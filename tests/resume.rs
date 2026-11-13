use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
enum Mode {
    IgnoreRange,
    HonorRange,
    Unsatisfiable,
}

fn temp_path(name: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("rget-test-{}-{}-{name}", std::process::id(), id));
    let _ = fs::remove_file(&path);
    path
}

fn spawn_server(body: Vec<u8>, mode: Mode) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/file.bin", listener.local_addr().unwrap());

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
        let start = request
            .split("\r\n")
            .find_map(|line| line.strip_prefix("range: bytes="))
            .and_then(|value| value.split('-').next())
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);

        let response = match mode {
            Mode::IgnoreRange => {
                let mut response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Disposition: attachment; filename=\"Example App.apk\"\r\nContent-Type: application/vnd.android.package-archive\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .into_bytes();
                response.extend_from_slice(&body);
                response
            }
            Mode::HonorRange => {
                let start = start.min(body.len());
                let end = body.len().saturating_sub(1);
                let mut response = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nContent-Disposition: attachment; filename=\"Example App.apk\"\r\nContent-Type: application/vnd.android.package-archive\r\nConnection: close\r\n\r\n",
                    body.len(),
                    body.len() - start
                )
                .into_bytes();
                response.extend_from_slice(&body[start..]);
                response
            }
            Mode::Unsatisfiable => format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nContent-Disposition: attachment; filename=\"Example App.apk\"\r\nContent-Type: application/vnd.android.package-archive\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes(),
        };

        let _ = stream.write_all(&response);
        let _ = stream.flush();
    });

    (url, handle)
}

fn spawn_redirect_server(body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!(
        "http://{}/download.php?id=17068436&key=token",
        listener.local_addr().unwrap()
    );

    let handle = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];

            loop {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let request = String::from_utf8_lossy(&request);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");

            let response = if path.starts_with("/download.php") {
                b"HTTP/1.1 302 Found\r\nLocation: /com.truecaller_26.44.6-2644006_apkmirror.com.apkm?X-Amz=token\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
            } else if path.starts_with("/com.truecaller_26.44.6") {
                let mut response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/vnd.android.package-archive\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .into_bytes();
                response.extend_from_slice(&body);
                response
            } else {
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
            };

            let _ = stream.write_all(&response);
            let _ = stream.flush();
        }
    });

    (url, handle)
}

async fn run_download(
    url: &str,
    path: &Path,
) -> Result<rget::DownloadOutcome, rget::DownloadError> {
    rget::download(url, path, CancellationToken::new(), |_| {}).await
}

#[tokio::test]
async fn server_ignoring_range_restarts_instead_of_appending() {
    let body = b"0123456789".to_vec();
    let path = temp_path("ignore-range");
    fs::write(&path, &body[..4]).unwrap();

    let (url, server) = spawn_server(body.clone(), Mode::IgnoreRange);
    let outcome = run_download(&url, &path).await.unwrap();
    server.join().unwrap();

    assert!(!outcome.already_complete);
    assert_eq!(outcome.filename, "Example App.apk");
    assert_eq!(
        outcome.content_type.as_deref(),
        Some("application/vnd.android.package-archive")
    );
    assert_eq!(fs::read(&path).unwrap(), body);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn filename_comes_from_final_redirect_url() {
    let body = b"apk-bytes".to_vec();
    let path = temp_path("redirect-name");

    let (url, server) = spawn_redirect_server(body.clone());
    let outcome = run_download(&url, &path).await.unwrap();
    server.join().unwrap();

    assert_eq!(
        outcome.filename,
        "com.truecaller_26.44.6-2644006_apkmirror.com.apkm"
    );
    assert_eq!(fs::read(&path).unwrap(), body);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn complete_file_is_not_rewritten_when_server_ignores_range() {
    let body = b"0123456789".to_vec();
    let path = temp_path("complete-ignore-range");
    fs::write(&path, &body).unwrap();

    let (url, server) = spawn_server(body.clone(), Mode::IgnoreRange);
    let outcome = run_download(&url, &path).await.unwrap();
    server.join().unwrap();

    assert!(outcome.already_complete);
    assert_eq!(outcome.downloaded, body.len() as u64);
    assert_eq!(fs::read(&path).unwrap(), body);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn complete_file_is_detected_from_416_response() {
    let body = b"0123456789".to_vec();
    let path = temp_path("complete-416");
    fs::write(&path, &body).unwrap();

    let (url, server) = spawn_server(body.clone(), Mode::Unsatisfiable);
    let outcome = run_download(&url, &path).await.unwrap();
    server.join().unwrap();

    assert!(outcome.already_complete);
    assert_eq!(outcome.downloaded, body.len() as u64);
    assert_eq!(fs::read(&path).unwrap(), body);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn partial_content_resumes_from_existing_bytes() {
    let body = b"0123456789".to_vec();
    let path = temp_path("honor-range");
    fs::write(&path, &body[..4]).unwrap();

    let (url, server) = spawn_server(body.clone(), Mode::HonorRange);
    let outcome = run_download(&url, &path).await.unwrap();
    server.join().unwrap();

    assert!(!outcome.already_complete);
    assert_eq!(outcome.downloaded, body.len() as u64);
    assert_eq!(fs::read(&path).unwrap(), body);
    let _ = fs::remove_file(path);
}
