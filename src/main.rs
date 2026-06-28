use std::net::{
    TcpListener, TcpStream
};
use std::io::{
    BufReader, BufRead, Read, Write
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

impl TryFrom<&mut TcpStream> for Request {
    type Error = Box<dyn std::error::Error>;

    fn try_from(stream: &mut TcpStream) -> Result<Self, Self::Error> {
        let mut reader = BufReader::new(stream);

        let mut line = String::new();
        reader.read_line(&mut line)?;

        let request_line = line.trim_end().to_string();
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err("Invalid request line: incomplete format.".into());
        }

        let method = parts[0].to_string();
        let path = parts[1].to_string();
        let version = parts[2].to_string();

        let mut headers = HashMap::new();
        loop {
            let mut header = String::new();
            reader.read_line(&mut header)?;

            let header = header.trim_end();

            if header.is_empty() {
                break;
            }

            if let Some((key, value)) = header.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        let content_length: usize = headers
            .get("Content-Length")
            .and_then(|val| val.parse().ok())
            .unwrap_or(0);

        let body = if content_length > 0 {
            let mut buf = vec![0; content_length];
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

    let request = Request::try_from(&mut stream)?;

    let response = if request.method == "GET" && request.path == "/" {
        String::from("HTTP/1.1 200 OK\r\n\r\n")
    } else if request.method == "GET" && request.path == String::from("/echo") || request.path.starts_with("/echo/") {
        let content = request.path.strip_prefix("/echo/").unwrap_or("");
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            content.len(), content
        )
    } else if request.method == "GET" && request.path == "/user-agent" || request.path == "/user-agent/" {
        let user_agent = request.headers.get("User-Agent").map(|s| s.as_str()).unwrap_or("");
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            user_agent.len(), user_agent
        )
    } else if request.method == "GET" && request.path.starts_with("/files/") {
        let file_path = match request.path.strip_prefix("/files/") {
            Some(path) => path,
            None => {
                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
                stream.flush()?;
                return Ok(());
            }
        };

        let base = std::fs::canonicalize("./public")?;
        let full_path = base.join(file_path);

        let canonical = match std::fs::canonicalize(&full_path) {
            Ok(p) => p,
            Err(_) => {
                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
                stream.flush()?;
                return Ok(());
            }
        };

        if !canonical.starts_with(&base) {
            stream.write_all(b"HTTP/1.1 403 Forbidden\r\n\r\n")?;
            stream.flush()?;
            return Ok(());
        }

        let body = match std::fs::read_to_string(&canonical) {
            Ok(content) => content,
            Err(_) => {
                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
                stream.flush()?;
                return Ok(());
            }
        };

        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    } else if request.method == "POST" && request.path.starts_with("/files/") {
        let file_name = match request.path.strip_prefix("/files/") {
            Some(path) => path,
            None => {
                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
                stream.flush()?;
                return Ok(());
            }
        };

        let base = std::fs::canonicalize("./public")?;
        let file_path = base.join(file_name);

        let body = match request.body {
            Some(content) => content,
            None => {
                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
                stream.flush()?;
                return Ok(());
            }
        };

        std::fs::write(file_path, body)?;
        String::from("HTTP/1.1 201 Created\r\n\r\n")
    } else {
        String::from("HTTP/1.1 404 Not Found\r\n\r\n")
    };

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
