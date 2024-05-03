use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use thrift::protocol::{TCompactInputProtocol, TCompactOutputProtocol};
use thrift::transport::{TIoChannel, TTcpChannel};

use crate::stub::{TWfsIfaceSyncClient, WfsAck, WfsAuth, WfsData, WfsFile, WfsIfaceSyncClient};

struct Client {
    wfsconn: Box<dyn TWfsIfaceSyncClient>,
    host_port: String,
    tls: bool,
    auth: Box<WfsAuth>,
    // count: Mutex<i32>,
    ping_num: i32,
}

impl Client {
    fn new(tls: bool, host: &str, port: &i32, name: &str, pwd: &str) -> Option<Client> {
        let host_port = format!("{}:{}", host, port);
        let mut ttcpchannel = TTcpChannel::new();
        let result = ttcpchannel.open(host_port.clone());
        match result {
            Ok(_s) => {
                let (i_chan, o_chan) = ttcpchannel.split().unwrap();
                let i_prot = TCompactInputProtocol::new(i_chan);
                let o_prot = TCompactOutputProtocol::new(o_chan);
                let wfsconn = Box::new(WfsIfaceSyncClient::new(i_prot, o_prot));
                let wc = Client {
                    wfsconn: wfsconn,
                    host_port: host_port,
                    tls: tls,
                    auth: Box::new(WfsAuth::new(name.to_string(), pwd.to_string())),
                    ping_num: 0,
                };
                return Some(wc);
            }
            Err(e) => {
                println!("open connect error:{}", e);
                return None;
            }
        }
    }

    fn link(&mut self) -> Option<WfsAck> {
        let mut ttcpchannel = TTcpChannel::new();
        let result = ttcpchannel.open(self.host_port.to_string());
        match result {
            Ok(_s) => {
                let (i_chan, o_chan) = ttcpchannel.split().unwrap();
                let i_prot = TCompactInputProtocol::new(i_chan);
                let o_prot = TCompactOutputProtocol::new(o_chan);
                self.wfsconn = Box::new(WfsIfaceSyncClient::new(i_prot, o_prot));
                self.ping_num = 0;
                return self.auth();
            }
            Err(e) => {
                println!("open connect error:{}", e);
                return None;
            }
        }
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
            if wc_thread.ping_num > 5 {
                wc_thread.re_link();
                continue;
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

    // fn close(&mut self) {
    // self.transport.close();
    // }

    fn append(&mut self, file: WfsFile) -> WfsAck {
        let result = self.wfsconn.append(file);
        match result {
            Ok(value) => value,
            Err(_error) => WfsAck::new(false, None),
        }
    }

    fn ping(&mut self) -> i8 {
        let result = self.wfsconn.ping();
        match result {
            Ok(value) => {
                self.ping_num = 1;
                return value;
            }
            Err(_error) => 0,
        }
    }

    fn delete(&mut self, path: &str) -> WfsAck {
        let result = self.wfsconn.delete(path.to_string());
        match result {
            Ok(value) => value,
            Err(_error) => WfsAck::new(false, None),
        }
    }

    fn rename(&mut self, path: &str, new_path: &str) -> WfsAck {
        let result = self.wfsconn.rename(path.to_string(), new_path.to_string());
        match result {
            Ok(value) => value,
            Err(_error) => WfsAck::new(false, None),
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
            client.auth();
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
        wc_thread.append(file)
    }

    /**
     * delete file by path
     */
    pub fn delete(&mut self, path: &str) -> WfsAck {
        let mut wc_thread = self.c.lock().unwrap();
        wc_thread.delete(path)
    }

    /**
     *  rename file
     */
    pub fn rename(&mut self, path: &str, new_path: &str) -> WfsAck {
        let mut wc_thread = self.c.lock().unwrap();
        wc_thread.rename(path, new_path)
    }

    /**
     *  get fileData by path
     */
    pub fn get(&mut self, path: &str) -> Option<WfsData> {
        let mut wc_thread = self.c.lock().unwrap();
        wc_thread.get(path)
    }
}
