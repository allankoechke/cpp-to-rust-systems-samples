use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    loop {
        let mut request_line = String::new();
        if reader.read_line(&mut request_line)? == 0 {
            return Ok(()); // client closed the connection
        }
        let mut parts = request_line.split_whitespace();
        let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        let (mut keep_alive, mut content_length) = (true, 0u64);
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let line = line.trim_end();
            if line.is_empty() { break; } // blank line ends the headers
            if let Some((k, v)) = line.split_once(':') {
                let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
                if k == "connection" && v.eq_ignore_ascii_case("close") { keep_alive = false; }
                if k == "content-length" { content_length = v.parse().unwrap_or(0); }
            }
        }
        std::io::copy(&mut (&mut reader).take(content_length), &mut std::io::sink())?; // drain body
        let body = format!("you asked for {method} {path}\n");
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n{}",
            body.len(),
            if keep_alive { "keep-alive" } else { "close" },
            body
        )?;
        if !keep_alive { return Ok(()); }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    for stream in listener.incoming() {
        let stream = stream?;
        thread::spawn(move || { let _ = handle(stream); }); // thread per connection: your baseline
    }
    Ok(())
}
