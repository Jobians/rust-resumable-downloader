use std::env;
use std::fs::{metadata, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use futures_util::StreamExt;
use reqwest::header::{CONTENT_LENGTH, RANGE};

fn to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn to_kb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0
}

fn format_eta(seconds: f64) -> String {
    let secs = seconds.round() as u64;
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{:02}h:{:02}m:{:02}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{:02}m:{:02}s", minutes, seconds)
    } else {
        format!("{:02}s", seconds)
    }
}

fn print_usage() {
    eprintln!("🔹 {} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    eprintln!("📦 {}", env!("CARGO_PKG_DESCRIPTION"));
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  rget <url> [output_path]");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  rget https://example.com/file.zip");
    eprintln!("  rget https://example.com/file.zip ./downloads/file.zip");
    eprintln!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        std::process::exit(0);
    }

    let url = &args[1];

    let path = if args.len() >= 3 {
        args[2].clone()
    } else {
        url.split('/')
            .last()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                eprintln!("❌ Could not determine filename from URL. Please provide an explicit output path.");
                print_usage();
                std::process::exit(1);
            })
    };

    let path_obj = Path::new(&path);
    let start = metadata(&path_obj).map(|m| m.len()).unwrap_or(0);

    println!("📥 Starting download from: {}", url);
    if start > 0 {
        println!("🔄 Resuming from byte {}", start);
    } else {
        println!("🆕 Starting new download");
    }

    let client = reqwest::Client::new();
    let mut request = client.get(url);
    if start > 0 {
        request = request.header(RANGE, format!("bytes={}-", start));
    }

    let response = request.send().await?;
    if !response.status().is_success() && response.status().as_u16() != 206 {
        eprintln!("❌ Server returned HTTP {}", response.status());
        std::process::exit(1);
    }

    let total_size = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v + start)
        .unwrap_or(start);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path_obj)?;

    println!("📄 Saving to: {}", path_obj.display());

    let mut downloaded = start;
    let mut stream = response.bytes_stream();
    let start_time = Instant::now();
    
    let mut last_check = Instant::now();
    let mut last_downloaded = downloaded;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        let now = Instant::now();
        let interval = now.duration_since(last_check).as_secs_f64();
        let delta = downloaded - last_downloaded;
        
        let speed_str = if interval > 0.0 {
            if delta == 0 {
                "…".to_string()
            } else if to_mb(delta) / interval >= 1.0 {
                format!("{:.2} MB/s", to_mb(delta) / interval)
            } else {
                format!("{:.0} KB/s", to_kb(delta) / interval)
            }
        } else {
            "…".to_string()
        };
        
        if interval >= 1.0 {
            last_check = now;
            last_downloaded = downloaded;
        }

        let percent = downloaded as f64 / total_size as f64 * 100.0;
        let eta = if delta > 0 {
            (total_size - downloaded) as f64 / (delta as f64 / interval)
        } else {
            0.0
        };

        print!("\r\x1b[2K");
        print!(
            "{:.2}% ({:.2}/{:.2} MB) {} | ETA: {}",
            percent,
            to_mb(downloaded),
            to_mb(total_size),
            speed_str,
            format_eta(eta)
        );
        io::stdout().flush()?;
    }

    println!(
        "\n✅ Download complete in {:.2}s!",
        start_time.elapsed().as_secs_f64()
    );

    Ok(())
}
