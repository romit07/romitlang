use crate::{build_ui, default_out_dir, format_diagnostics};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub fn serve_preview(source_path: &Path, port: u16) -> Result<(), String> {
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("cannot read {}: {e}", source_path.display()))?;
    let file = source_path.to_string_lossy().to_string();
    let out_dir = default_out_dir(source_path);

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).map_err(|e| format!("bind {addr}: {e}"))?;
    eprintln!("Romit UI preview at http://{addr}/");
    eprintln!("(rebuilds on each request; edit {file} and refresh)");

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let source_path = source_path.to_path_buf();
        let source = source.clone();
        let file = file.clone();
        let out_dir = out_dir.clone();
        if let Err(diags) = build_ui(&source, &file, &out_dir) {
            let body = format!("<pre>{}</pre>", html_escape(&format_diagnostics(&diags)));
            let _ = write_http(&mut stream, 500, "text/html", body.as_bytes());
            continue;
        }
        let _ = source_path;
        if let Err(e) = handle_request(&mut stream, &out_dir) {
            eprintln!("request error: {e}");
        }
    }
    Ok(())
}

fn handle_request(stream: &mut TcpStream, out_dir: &Path) -> std::io::Result<()> {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");

    let (status, file_path, content_type) = match path {
        "/" | "/index.html" => (200, out_dir.join("index.html"), "text/html"),
        "/style.css" => (200, out_dir.join("style.css"), "text/css"),
        _ => {
            let body = b"Not Found";
            return write_http(stream, 404, "text/plain", body);
        }
    };

    let body = fs::read(&file_path).unwrap_or_default();
    write_http(stream, status, content_type, &body)
}

fn write_http(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    thread::sleep(Duration::from_millis(10));
    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn pick_port() -> u16 {
    3847
}
