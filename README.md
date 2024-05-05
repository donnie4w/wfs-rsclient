# wfs-rsclient

###### Rust Client for WFS

------------

###### 配置 wfs-rsclient 依赖

    [dependencies]
    wfs="0.0.1"

------------


###### 引入wfs-rsclient库

    use wfs::{client::WfsClient, stub::WfsFile};

------------

###### 创建wfsclient实例对象

	let mut wc = WfsClient::new(false, "127.0.0.1", 6802, "admin", "123").unwrap();

###### 参数说明

1. 第一个参数：是否TLS
2. 第二个参数：wfs thrift 服务ip或域名
3. 第三个参数：端口
4. 第四个参数：后台用户名
5. 第五个参数：后台密码

###### 上传文件

    let wa = wc.append(wfs_file);
    println!("{}", wa.ok);
    if !wa.ok {
        match wa.error {
            Some(value) => {
              let code = value.code.unwrap();
              println!("error code: {}", code);
             }
            None => println!("No value"),
         }
    }

###### 拉取文件

    let opdata = wc.get("readme1.md");
    match opdata {
        Some(value) => {
            let data = value.data.unwrap();
            println!("data length {}", data.len());
        }
        None => println!("No value"),
    }

###### 删除文件

    let wa = wc.delete("readme1.md");
    println!("delete ack status: {}", wa.ok);
    if !wa.ok {
        match wa.error {
            Some(value) => {
                let code = value.code.unwrap();
                println!("error code: {}", code);
            }
            None => println!("No value"),
        }
    }

###### 重命名

    let wa = wc.rename("readme1.md", "readme2.md");
    println!("rename ack status:{}", wa.ok);
    if !wa.ok {
        match wa.error {
            Some(value) => {
                let code = value.code.unwrap();
                println!("error code: {}", code);
            }
            None => println!("No value"),
        }
    }
