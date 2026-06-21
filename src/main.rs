use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};

fn handle_request(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    println!("Accepted request from: {}", stream.peer_addr()?);

    let mut buf_reader = BufReader::new(&mut stream);

    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line)?;

    let mut headers: Vec<String> = Vec::new();
    for line_result in buf_reader.lines() {
        let header_line = line_result?;
        if header_line.is_empty() {
            break;
        }

        headers.push(header_line);
    }

    let request_parts: Vec<&str> = request_line.split_whitespace().collect();
    let response = match request_parts[..] {
        ["GET", path, "HTTP/1.1"] => {
            if path == "/" {
                String::from("HTTP/1.1 200 OK\r\n\r\n")
            } else if path == "/echo" || path.starts_with("/echo/") {
                let content = path.strip_prefix("/echo/").unwrap_or("");
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    content.len(), content
                )
            } else if path == "/user-agent" || path == "/user-agent/" {
                let content = headers.iter()
                    .find_map(|h| h.strip_prefix("User-Agent: "))
                    .unwrap_or("unknown user agent");

                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    content.len(), content
                )
            } else {
                String::from("HTTP/1.1 404 Not Found\r\n\r\n")
            }
        }
        _ => String::from("HTTP/1.1 400 Bad Request\r\n\r\n")
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
