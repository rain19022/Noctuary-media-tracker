use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub form: HashMap<String, String>,
}

pub struct HttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub location: Option<String>,
}

impl HttpResponse {
    pub fn html(body: String) -> Self {
        Self {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: body.into_bytes(),
            location: None,
        }
    }

    pub fn redirect(path: &str) -> Self {
        Self {
            status: 303,
            content_type: "text/html; charset=utf-8",
            body: Vec::new(),
            location: Some(path.to_string()),
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: 404,
            content_type: "text/html; charset=utf-8",
            body: b"not found".to_vec(),
            location: None,
        }
    }

    pub fn static_file(content_type: &'static str, data: Vec<u8>) -> Self {
        Self {
            status: 200,
            content_type,
            body: data,
            location: None,
        }
    }
}

pub fn run<F>(addr: &str, mut handler: F) -> std::io::Result<()>
where
    F: FnMut(&HttpRequest) -> HttpResponse,
{
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        let mut stream = stream?;
        let _ = handle_connection(&mut stream, &mut handler);
    }
    Ok(())
}

fn handle_connection<F>(stream: &mut TcpStream, handler: &mut F) -> std::io::Result<()>
where
    F: FnMut(&HttpRequest) -> HttpResponse,
{
    let request = match read_request(stream) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    let response = handler(&request);
    write_response(stream, &response)
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<HttpRequest> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let raw = String::from_utf8_lossy(&buf[..n]);
    let mut lines = raw.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let full_path = parts.next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }

    let (path, query) = split_path_query(&full_path);
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(n);
    let body = &raw[body_start..];
    let form = if method == "POST" && content_length > 0 {
        parse_form(&body[..body.len().min(content_length)])
    } else {
        HashMap::new()
    };

    Ok(HttpRequest {
        method,
        path,
        query,
        form,
    })
}

fn split_path_query(full: &str) -> (String, HashMap<String, String>) {
    if let Some((path, q)) = full.split_once('?') {
        (path.to_string(), parse_query(q))
    } else {
        (full.to_string(), HashMap::new())
    }
}

fn parse_query(q: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in q.split('&') {
        if pair.is_empty() {
            continue;
        }
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(url_decode(k), url_decode(v));
        } else {
            map.insert(url_decode(pair), String::new());
        }
    }
    map
}

fn parse_form(body: &str) -> HashMap<String, String> {
    parse_query(body)
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(byte) = u8::from_str_radix(&format!("{h1}{h2}"), 16) {
                out.push(byte as char);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> std::io::Result<()> {
    let status_text = match response.status {
        200 => "OK",
        303 => "See Other",
        404 => "Not Found",
        _ => "OK",
    };
    let mut headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        status_text,
        response.content_type,
        response.body.len()
    );
    if let Some(loc) = &response.location {
        headers.push_str(&format!("Location: {loc}\r\n"));
    }
    headers.push_str("\r\n");
    stream.write_all(headers.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_query_string() {
        let q = parse_query("title=Solo%20Leveling&sort=rating");
        assert_eq!(q.get("title").unwrap(), "Solo Leveling");
        assert_eq!(q.get("sort").unwrap(), "rating");
    }
}
