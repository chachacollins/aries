use openssl::ssl::{SslMethod, SslConnector,SslVerifyMode};
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn make_request(url: &str) -> Result<String, String> {
    if !url.starts_with("gemini://") {
        return Err("give a url with gemini protocal prefix".to_string());
    }
    let hostname = match url.strip_prefix("gemini://").unwrap().split('/').next() {
        Some(h) => h,
        None => return Err("hostname does not have a terminating slash".to_string()),
    };
    let mut configure = match SslConnector::builder(SslMethod::tls()) {
        Ok(s) => s,
        Err(_) => return Err("failed to create ssl connector builder".to_string()),
    };
    //NOTE: gemini uses TOFU which I'm too lazy to implement
    configure.set_verify(SslVerifyMode::NONE);
    let connector = configure.build();
    let stream = match TcpStream::connect(format!("{hostname}:1965")) {
        Ok(s) => s,
        Err(_) => return Err("failed to connect to host".to_string()),
    };
    let mut stream = match connector.connect(hostname, stream) {
        Ok(s) => s,
        Err(_) => return Err("failed to create ssl connection".to_string()),
    };
    stream.write_all(format!("{url}\r\n").as_bytes()).unwrap();
    let mut res = vec![];
    stream.read_to_end(&mut res).unwrap();
    Ok(String::from_utf8_lossy(&res).to_string())
}
