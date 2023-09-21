use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::fs;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:3221").await.unwrap();
    println!("Listening on port 3221");
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                println!("New Connection");
                tokio::spawn(async move { 
                    handle_connection(&mut stream).await 
                });
            }
            Err(e) => { println!("Error: {}", e); }
        }
    }
}

static OK: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";
static ERROR: &[u8] = b"HTTP/1.1 404 Not Found\r\n\r\n";

async fn handle_connection(stream: &mut TcpStream) {
    let request = &mut [0; 512];
    stream.read(request).await.unwrap();

    let request = String::from_utf8_lossy(request);
    let request_parts: Vec<&str> = request.split("\r\n").collect();

    let requst_payload = payload_parser(&request_parts[1..]);
    let request_data: Vec<&str> = request_parts[0].split_ascii_whitespace().collect();
    let (method, path) = (request_data[0], request_data[1]);

    match (method, path) {
        ("GET", "/") => { empty_response(stream, OK).await },
        ("GET", "/user-agent") => { 
            let data = requst_payload.get("User-Agent").unwrap();
            body_response(stream, data, "text/plain").await;
        },
        ("GET", path) if path.starts_with("/echo/") => {
            let data = path.trim_start_matches("/echo/");
            body_response(stream, &data.to_string(), "text/plain").await
        },
        ("GET", path) if path.starts_with("/files/") => {
            let file_path = path.trim_start_matches("/files/");
            let file = fs::read_to_string(file_path).await;
            match file {
                Ok(contents) => { 
                    body_response(stream, &contents, "application/octet-stream").await; 
                }, Err(_) => { empty_response(stream, ERROR).await; }
            }
        },
        ("POST", path) if path.starts_with("/files/") => {
            let file_path = path.trim_start_matches("/files/");
            let file_data = requst_payload.get("post_payload").unwrap();
            let file = fs::write(file_path, file_data).await;
            match file {
                Ok(_) => { 
                    empty_response(stream, b"HTTP/1.1 201 OK\r\n\r\n").await; 
                }, Err(_) => { empty_response(stream, ERROR).await; }
            }
        },
        _ => { empty_response(stream, ERROR).await; }
    };
}

fn payload_parser(payload: &[&str]) -> HashMap<String, String> {
    let mut payload_hashmap = HashMap::new();
    for part in payload {
        if let Some(p) = part.split_once(": ") {
            payload_hashmap.insert(p.0.to_string(), p.1.to_string());
        } else {
            if !part.is_empty() {
                let cleaned = part.trim_matches('\0'); 
                payload_hashmap.insert(
                    "post_payload".to_string(), 
                    cleaned.to_string()
                );
            }
        }
    }
    payload_hashmap
}

async fn empty_response(stream: &mut TcpStream, response: &[u8]) {
    stream.write_all(response).await.unwrap();
}

async fn body_response(stream: &mut TcpStream, data: &String, contet_type: &str) {
    stream.write_all(format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}", 
        contet_type, data.len(), data).as_bytes()
    ).await.unwrap();
}