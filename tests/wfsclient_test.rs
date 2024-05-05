use std::{
    fs::File,
    io::{Error, Read},
    path::Path,
};

use wfs::{client::WfsClient, stub::WfsFile};

// 从指定路径读取文件并创建一个 WfsFile 实例
// file_path 文件的路径
// name  可选的自定义文件名，默认为空字符串时从路径中提取
// 成功时返回 WfsFile 实例，失败时返回 io::Error
pub fn read_file_to_wfsfile<P: AsRef<Path>>(
    file_path: P,
    mut name: String,
) -> Result<WfsFile, Error> {
    // 如果用户未提供文件名，则从路径中提取
    if name.is_empty() {
        name = file_path
            .as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
    }

    let mut file = File::open(file_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    Ok(WfsFile {
        data,
        name,
        compress: None,
    })
}

#[test]
fn test_append() {
    let mut wc = WfsClient::new(true, "192.168.2.11", 6802, "admin", "123").unwrap();
    match read_file_to_wfsfile("Cargo.toml", "12345.toml".to_string()) {
        Ok(wfs_file) => {
            println!("文件名: {}", wfs_file.name);
            println!("数据长度: {}", wfs_file.data.len());
            // 打印压缩选项，如果有的话
            if let Some(level) = wfs_file.compress {
                println!("压缩级别: {}", level);
            }
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
        }
        Err(e) => eprintln!("文件读取错误: {}", e),
    }
}

#[test]
fn test_get() {
    let mut wc = WfsClient::new(true, "192.168.2.11", 6802, "admin", "123").unwrap();
    let opdata = wc.get("readme2.md");
    match opdata {
        Some(value) => {
            let data = value.data.unwrap();
            println!("data length {}", data.len());
        }
        None => println!("No value"),
    }
}

#[test]
fn test_delete() {
    let mut wc = WfsClient::new(false, "192.168.2.11", 6802, "admin", "123").unwrap();
    let wa = wc.delete("123.toml");
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
}

#[test]
fn test_rename() {
    let mut wc = WfsClient::new(false, "192.168.2.11", 6802, "admin", "123").unwrap();
    let wa = wc.rename("README.md", "readme1.md");
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
}
