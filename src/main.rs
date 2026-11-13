use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process;
use std::time::Instant;

use rget::{DownloadError, Progress, download, format_size};
use tokio_util::sync::CancellationToken;

enum Command {
    Download { url: String, output: Option<String> },
    Help,
    Version,
}

fn parse_args() -> Result<Command, String> {
    let mut positional = Vec::new();
    let mut options_done = false;

    for arg in env::args().skip(1) {
        if !options_done {
            match arg.as_str() {
                "-h" | "--help" => return Ok(Command::Help),
                "-V" | "--version" => return Ok(Command::Version),
                "--" => {
                    options_done = true;
                    continue;
                }
                value if value.starts_with('-') => {
                    return Err(format!("unexpected option '{value}'"));
                }
                _ => {}
            }
        }

        positional.push(arg);
    }

    if positional.is_empty() {
        return Err("missing URL".to_string());
    }
    if positional.len() > 2 {
        return Err(format!("unexpected argument '{}'", positional[2]));
    }

    let mut args = positional.into_iter();
    let url = args.next().expect("URL is present");
    let output = args.next();

    Ok(Command::Download { url, output })
}

fn print_help() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!("{}", env!("CARGO_PKG_DESCRIPTION"));
    println!();
    println!("Usage:");
    println!("  rget [OPTIONS] <URL> [OUTPUT]");
    println!();
    println!("Arguments:");
    println!("  <URL>     URL to download");
    println!("  [OUTPUT]  Output path (defaults to the filename in the URL)");
    println!();
    println!("Options:");
    println!("  -h, --help     Show this help");
    println!("  -V, --version  Show version");
}

fn exit_usage_error(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!(
        "Try '{} --help' for more information.",
        env!("CARGO_PKG_NAME")
    );
    process::exit(2);
}

fn default_output_path(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let filename = path.rsplit('/').next()?;

    (!filename.is_empty()).then(|| filename.to_string())
}

fn clear_progress_line() {
    if io::stderr().is_terminal() {
        eprint!("\r\x1b[2K");
    }
}

fn print_progress(progress: &Progress) {
    if !io::stderr().is_terminal() {
        return;
    }

    let mut stderr = io::stderr().lock();
    let _ = write!(stderr, "\r\x1b[2K");

    if progress.total > 0 {
        let percent = progress.percent().unwrap_or(0.0);
        let _ = write!(
            stderr,
            "{:>6.2}%  {}  {}  ETA {}",
            percent,
            progress.formatted_size(),
            progress.formatted_speed(),
            progress.formatted_eta()
        );
    } else {
        let _ = write!(
            stderr,
            "{}  {}",
            progress.formatted_size(),
            progress.formatted_speed()
        );
    }

    let _ = stderr.flush();
}

#[tokio::main]
async fn main() {
    let command = match parse_args() {
        Ok(command) => command,
        Err(message) => exit_usage_error(&message),
    };

    let (url, output) = match command {
        Command::Help => {
            print_help();
            return;
        }
        Command::Version => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            return;
        }
        Command::Download { url, output } => (url, output),
    };

    let path = output
        .or_else(|| default_output_path(&url))
        .unwrap_or_else(|| {
            exit_usage_error("could not determine an output filename; provide OUTPUT explicitly")
        });
    let path = Path::new(&path);
    let existing = std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    eprintln!("URL:    {url}");
    eprintln!("Output: {}", path.display());
    if existing > 0 {
        eprintln!("Resuming from {}", format_size(existing));
    }

    let started = Instant::now();
    let cancel = CancellationToken::new();
    let result = download(&url, path, cancel, |progress| print_progress(&progress)).await;

    clear_progress_line();

    match result {
        Ok(outcome) if outcome.already_complete => {
            eprintln!(
                "Already complete: {} ({})",
                path.display(),
                format_size(outcome.downloaded)
            );
        }
        Ok(outcome) => {
            eprintln!(
                "Downloaded to {} ({}) in {:.2}s",
                path.display(),
                format_size(outcome.downloaded),
                started.elapsed().as_secs_f64()
            );
        }
        Err(DownloadError::Cancelled) => {
            eprintln!("Download cancelled.");
            process::exit(130);
        }
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }
}
