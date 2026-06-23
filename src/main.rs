use std::net::{
    TcpListener, TcpStream
};
use std::io::{
    BufReader, BufRead, Write
};
use std::collections::HashMap;
use std::convert::TryFrom;

struct Request {
    method: String,
    path: String,
    version: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl TryFrom<TcpStream> for Request {
    type Error = std::error::Error;

    fn try_from(stream: TcpStream) -> Result<Self, Self::Error> {
        let mut reader = BufReader::new(&mut stream);

        let mut line = String::new();
        reader.read_line(&mut line)?;

        let request_line = line.trim_end().to_string();
        let mut request_line_parts = request_line.split_whitespace();

        let method = request_line_parts.next().unwrap().to_string();
        let path = request_line_parts.next().unwrap().to_string();
        let version = request_line_parts.next().unwrap().to_string();

        let mut headers = HashMap::new();
        loop {
            let mut header = String::new();
            reader.read_line(&mut header)?;

            let header = header.trim_end();

            if header.is_empty() {
                break;
            }

            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        let content_length: usize = headers
            .get("Content-Length")
            .and_then(|val| val.parse().ok())
            .unwrap_or(0);

        let body = if content_length > 0 {
            let mut buf = vec![0u8; content_length];
            reader.read_exact(&mut buf)?;

            Some(String::from_utf8_lossy(&buf).to_string())
        } else {
            None
        };

        Ok(Request {
            method, path, version, headers, body
        })
    }
}

fn handle_request(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    println!("Accepted request from: {}", stream.peer_addr()?);

    let request = Request::TryFrom(streami)?;
    let response = if request.path == String::from("/") {
        String::from("HTTP/1.1 200 OK\r\n\r\n")
    } else if request.path == String::from("/echo") || request.path.starts_with("/echo/") {
        let content = request.path.strip_prefix("/echo/").unwrap_or("");
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            content.len(), content
        )
    } else if request.path == String::from("/user-agent") || request.path.starts_with("/user-agent/") {
        let user_agent = request.headers.get("User-Agent: ")

        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            user_agent.len(), user_agent
        )
    } else {
        String::from("HTTP/1.1 404 Not Found\r\n\r\n")
    }

    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn run() -> Result<(), std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    println!("Server running at {}", listener.local_addr()?);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    if let Err(e) = handle_request(stream) {
                        eprintln!("Failed to handle request: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("Error: {}", e)
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
}
