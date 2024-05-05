use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use native_tls::{TlsConnector, TlsStream};
use thrift::protocol::{TCompactInputProtocol, TCompactOutputProtocol};
use thrift::transport::{TIoChannel, TTcpChannel};

use crate::stub::{
    TWfsIfaceSyncClient, WfsAck, WfsAuth, WfsData, WfsError, WfsFile, WfsIfaceSyncClient,
};
struct Client {
    wfsconn: Box<dyn TWfsIfaceSyncClient>,
    host: String,
    post: i32,
    tls: bool,
    auth: Box<WfsAuth>,
    ping_num: i32,
    close: bool,
}

struct TRead {
    input: Arc<Mutex<TlsStream<TcpStream>>>,
}

impl Read for TRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut stream = self.input.lock().unwrap();
        return stream.read(buf);
    }
}

struct TWrite {
    output: Arc<Mutex<TlsStream<TcpStream>>>,
}

impl Write for TWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut stream = self.output.lock().unwrap();
        return stream.write(buf);
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut stream = self.output.lock().unwrap();
        return stream.flush();
    }
}

impl Client {
    fn new(tls: bool, host: &str, port: &i32, name: &str, pwd: &str) -> Option<Self> {
        let host_port = format!("{}:{}", host, port);
        let wfsconn: Box<dyn TWfsIfaceSyncClient>;
        if tls {
            let tls_connector = Self::create_tls_connector().ok()?;
            let tcp_stream = TcpStream::connect(host_port.clone());
            if tcp_stream.is_err() {
                match tcp_stream {
                    Ok(_v) => {}
                    Err(e) => {
                        println!("open connect error:{}", e)
                    }
                }
                return None;
            }
            let tls_stream = tls_connector
                .connect(host_port.as_str(), tcp_stream.unwrap())
                .ok()?;

            let arc_tls_stream = Arc::new(Mutex::new(tls_stream));
            let tread = TRead {
                input: arc_tls_stream.clone(),
            };
            let twrite = TWrite {
                output: arc_tls_stream.clone(),
            };
            let i_prot = TCompactInputProtocol::new(tread);
            let o_prot = TCompactOutputProtocol::new(twrite);
            wfsconn = Box::new(WfsIfaceSyncClient::new(i_prot, o_prot));
        } else {
            let mut ttcpchannel = TTcpChannel::new();
            let result = ttcpchannel.open(host_port.clone());
            if result.is_err() {
                return None;
            }
            let (i_chan, o_chan) = ttcpchannel.split().unwrap();
            let i_prot = TCompactInputProtocol::new(i_chan);
            let o_prot = TCompactOutputProtocol::new(o_chan);
            wfsconn = Box::new(WfsIfaceSyncClient::new(i_prot, o_prot));
        }
        let wc = Client {
            wfsconn: wfsconn,
            host: host.to_string(),
            post: *port,
            tls: tls,
            auth: Box::new(WfsAuth::new(name.to_string(), pwd.to_string())),
            ping_num: 0,
            close: false,
        };
        return Some(wc);
    }

    fn link(&mut self) -> Option<WfsAck> {
        let auth = &self.auth;
        let c = Self::new(
            self.tls,
            &self.host.as_str(),
            &self.post,
            auth.name.as_ref().unwrap().as_str(),
            auth.pwd.as_ref().unwrap().as_str(),
        );
        if c.is_none() {
            return None;
        }
        let client = c.unwrap();
        self.wfsconn = client.wfsconn;
        return self.auth();
    }

    fn create_tls_connector() -> Result<TlsConnector, native_tls::Error> {
        let mut builder = TlsConnector::builder();
        builder.danger_accept_invalid_hostnames(true);
        builder.danger_accept_invalid_certs(true);
        builder.build()
    }

    fn auth(&mut self) -> Option<WfsAck> {
        let result = self.wfsconn.auth(*self.auth.clone());
        match result {
            Ok(value) => Some(value),
            Err(_error) => None,
        }
    }

    fn timer(wc_for_thread: Arc<Mutex<Client>>) {
        loop {
            thread::sleep(Duration::from_secs(3));
            let mut wc_thread = wc_for_thread.lock().unwrap();
            if wc_thread.close {
                break;
            }
            if wc_thread.ping_num > 3 {
                if wc_thread.re_link().is_some() {
                    wc_thread.ping_num = 0;
                }
            } else {
                wc_thread.ping_num += 1;
                let result = {
                    let ping_result = wc_thread.ping();
                    ping_result
                };
                if result == 1 {
                    wc_thread.ping_num -= 1;
                }
            }
        }
    }

    fn close(&mut self) {
        self.close = true;
    }

    fn append(&mut self, file: WfsFile) -> WfsAck {
        let result = self.wfsconn.append(file);
        match result {
            Ok(value) => value,
            Err(_error) => null_wfs_ack(),
        }
    }

    fn ping(&mut self) -> i8 {
        let result = self.wfsconn.ping();
        match result {
            Ok(value) => {
                self.ping_num = 0;
                return value;
            }
            Err(_error) => 0,
        }
    }

    fn delete(&mut self, path: &str) -> WfsAck {
        let result = self.wfsconn.delete(path.to_string());
        match result {
            Ok(value) => value,
            Err(_error) => null_wfs_ack(),
        }
    }

    fn rename(&mut self, path: &str, new_path: &str) -> WfsAck {
        let result = self.wfsconn.rename(path.to_string(), new_path.to_string());
        match result {
            Ok(value) => value,
            Err(_error) => null_wfs_ack(),
        }
    }

    fn get(&mut self, path: &str) -> Option<WfsData> {
        let result = self.wfsconn.get(path.to_string());
        match result {
            Ok(data) => Some(data),
            Err(_error) => None,
        }
    }

    fn re_link(&mut self) -> Option<WfsAck> {
        match (self.auth.name.as_deref(), self.auth.pwd.as_deref()) {
            (Some(name), Some(pwd)) => {
                if !name.is_empty() && !pwd.is_empty() {
                    return self.link();
                } else {
                    return None;
                }
            }
            _ => None,
        }
    }
}

fn newclient(
    tls: bool,
    host: &str,
    port: i32,
    name: &str,
    pwd: &str,
) -> Option<Arc<Mutex<Client>>> {
    let wclient = Client::new(tls, host, &port, name, pwd);
    match wclient {
        Some(mut client) => {
            let ar = client.auth();
            match ar {
                Some(ar) => {
                    if !ar.ok {
                        let er = ar.error.unwrap();
                        println!("auth failed:{}", er.code.unwrap());
                        return None;
                    }
                }
                None => {
                    return None;
                }
            }

            let wc = Arc::new(Mutex::new(client));
            let wc_for_thread = Arc::clone(&wc);
            thread::spawn(move || {
                Client::timer(wc_for_thread);
            });
            Some(wc)
        }
        _ => None,
    }
}

pub struct WfsClient {
    c: Arc<Mutex<Client>>,
}

impl WfsClient {
    pub fn new(tls: bool, host: &str, port: i32, name: &str, pwd: &str) -> Option<WfsClient> {
        let wc = newclient(tls, host, port, name, pwd);
        match wc {
            Some(client) => {
                let wc = WfsClient { c: client };
                Some(wc)
            }
            _ => None,
        }
    }

    /**
     * upload file
     */
    pub fn append(&mut self, file: WfsFile) -> WfsAck {
        let mut wc_thread = self.c.lock().unwrap();
        if !wc_thread.close {
            wc_thread.append(file)
        } else {
            null_wfs_ack()
        }
    }

    /**
     * delete file by path
     */
    pub fn delete(&mut self, path: &str) -> WfsAck {
        let mut wc_thread = self.c.lock().unwrap();
        if !wc_thread.close {
            wc_thread.delete(path)
        } else {
            null_wfs_ack()
        }
    }

    /**
     *  rename file
     */
    pub fn rename(&mut self, path: &str, new_path: &str) -> WfsAck {
        let mut wc_thread = self.c.lock().unwrap();
        if !wc_thread.close {
            wc_thread.rename(path, new_path)
        } else {
            null_wfs_ack()
        }
    }

    /**
     *  get fileData by path
     */
    pub fn get(&mut self, path: &str) -> Option<WfsData> {
        let mut wc_thread = self.c.lock().unwrap();
        if !wc_thread.close {
            wc_thread.get(path)
        } else {
            return None;
        }
    }

    /**
     * close wfs client
     */
    pub fn close(&mut self) {
        let mut wc_thread = self.c.lock().unwrap();
        wc_thread.close();
    }
}

fn null_wfs_ack() -> WfsAck {
    return WfsAck {
        ok: false,
        error: Some(WfsError::new(0, "".to_string())),
    };
}
