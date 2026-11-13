use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::Response;
use reqwest::header::RANGE;
use tokio_util::sync::CancellationToken;

use crate::error::DownloadError;
use crate::filename::{response_content_type, response_filename};
use crate::progress::Progress;
use crate::resume::{ResumeAction, plan_resume, response_content_length, response_content_range};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct DownloadOutcome {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub already_complete: bool,
    pub filename: String,
    pub content_type: Option<String>,
}

pub async fn download<F>(
    url: &str,
    path: &Path,
    cancel: CancellationToken,
    mut on_progress: F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(Progress),
{
    let mut start = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => 0,
        Err(err) => return Err(err.into()),
    };

    let client = reqwest::Client::builder()
        .user_agent(concat!("rget/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let mut retried_without_range = false;

    loop {
        let mut request = client.get(url);
        if start > 0 {
            request = request.header(RANGE, format!("bytes={start}-"));
        }

        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(DownloadError::Cancelled),
            response = request.send() => response?,
        };

        let status = response.status();
        let content_length = response_content_length(&response);
        let content_range = response_content_range(&response);
        let content_type = response_content_type(&response);
        let filename = response_filename(&response, content_type.as_deref());
        let plan = plan_resume(start, status, content_length, content_range);

        match plan.action {
            ResumeAction::Reject => {
                return Err(if status.is_success() {
                    DownloadError::InvalidResponse(format!(
                        "unusable range response (status {status}, requested start {start})"
                    ))
                } else {
                    DownloadError::Http(status)
                });
            }
            ResumeAction::RetryWithoutRange => {
                if retried_without_range {
                    return Err(DownloadError::InvalidResponse(
                        "server rejected the retry without Range".to_string(),
                    ));
                }
                retried_without_range = true;
                start = 0;
                continue;
            }
            ResumeAction::AlreadyComplete => {
                let total = plan.total.unwrap_or(start);
                if start > total {
                    let file = OpenOptions::new().write(true).open(path)?;
                    file.set_len(total)?;
                }

                on_progress(Progress {
                    downloaded: total,
                    total,
                    speed_bps: 0.0,
                    eta_secs: Some(0.0),
                });

                return Ok(DownloadOutcome {
                    downloaded: total,
                    total: Some(total),
                    already_complete: true,
                    filename,
                    content_type,
                });
            }
            ResumeAction::Append => {
                let mut file = OpenOptions::new().create(true).append(true).open(path)?;
                let mut outcome = stream_response(
                    response,
                    &mut file,
                    start,
                    plan.total,
                    &cancel,
                    &mut on_progress,
                )
                .await?;
                outcome.filename = filename;
                outcome.content_type = content_type;
                return Ok(outcome);
            }
            ResumeAction::Restart => {
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)?;
                let mut outcome = stream_response(
                    response,
                    &mut file,
                    0,
                    plan.total,
                    &cancel,
                    &mut on_progress,
                )
                .await?;
                outcome.filename = filename;
                outcome.content_type = content_type;
                return Ok(outcome);
            }
        }
    }
}

async fn stream_response<F>(
    response: Response,
    file: &mut fs::File,
    start: u64,
    total: Option<u64>,
    cancel: &CancellationToken,
    on_progress: &mut F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(Progress),
{
    let total_for_progress = total.unwrap_or(0);
    let mut downloaded = start;
    let mut stream = response.bytes_stream();
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let mut last_sample = Instant::now();
    let mut last_bytes = downloaded;

    on_progress(Progress {
        downloaded,
        total: total_for_progress,
        speed_bps: 0.0,
        eta_secs: None,
    });

    loop {
        let next = tokio::select! {
            _ = cancel.cancelled() => {
                file.flush()?;
                return Err(DownloadError::Cancelled);
            }
            chunk = stream.next() => chunk,
        };

        let Some(chunk) = next else {
            break;
        };

        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        let now = Instant::now();
        if now.duration_since(last_emit) >= PROGRESS_INTERVAL {
            let interval = now.duration_since(last_sample).as_secs_f64();
            let delta = downloaded.saturating_sub(last_bytes);
            let speed_bps = if interval > 0.0 {
                delta as f64 / interval
            } else {
                0.0
            };
            let eta_secs = match total {
                Some(total) if speed_bps > 0.0 && total > downloaded => {
                    Some((total - downloaded) as f64 / speed_bps)
                }
                Some(_) => Some(0.0),
                None => None,
            };

            on_progress(Progress {
                downloaded,
                total: total_for_progress,
                speed_bps,
                eta_secs,
            });

            last_emit = now;
            last_sample = now;
            last_bytes = downloaded;
        }
    }

    file.flush()?;

    let elapsed = started.elapsed().as_secs_f64();
    let downloaded_this_run = downloaded.saturating_sub(start);
    let speed_bps = if elapsed > 0.0 {
        downloaded_this_run as f64 / elapsed
    } else {
        0.0
    };
    let final_total = total.unwrap_or(downloaded);

    on_progress(Progress {
        downloaded,
        total: final_total,
        speed_bps,
        eta_secs: Some(0.0),
    });

    if let Some(total) = total
        && downloaded != total
    {
        return Err(DownloadError::Incomplete { downloaded, total });
    }

    Ok(DownloadOutcome {
        downloaded,
        total,
        already_complete: false,
        filename: String::new(),
        content_type: None,
    })
}
