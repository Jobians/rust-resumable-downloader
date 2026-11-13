#[derive(Debug, Clone, Copy, Default)]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
    pub speed_bps: f64,
    pub eta_secs: Option<f64>,
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let units = ["KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64 / 1024.0;
    let mut index = 0;
    while value >= 1024.0 && index < units.len() - 1 {
        value /= 1024.0;
        index += 1;
    }

    if value >= 100.0 {
        format!("{value:.0} {}", units[index])
    } else {
        format!("{value:.1} {}", units[index])
    }
}

pub fn format_speed(bytes_per_second: f64) -> String {
    if !bytes_per_second.is_finite() || bytes_per_second <= 0.0 {
        return "—".to_string();
    }

    format!("{}/s", format_size(bytes_per_second.round() as u64))
}

pub fn format_eta(seconds: Option<f64>) -> String {
    let Some(seconds) = seconds else {
        return "—".to_string();
    };
    if !seconds.is_finite() || seconds < 0.0 {
        return "—".to_string();
    }

    let total_seconds = seconds.round() as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let remaining_seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m {remaining_seconds:02}s")
    } else {
        format!("{remaining_seconds}s")
    }
}

impl Progress {
    pub fn percent(&self) -> Option<f64> {
        if self.total == 0 {
            None
        } else {
            Some((self.downloaded as f64 / self.total as f64 * 100.0).clamp(0.0, 100.0))
        }
    }

    pub fn formatted_size(&self) -> String {
        if self.total > 0 {
            format!(
                "{} / {}",
                format_size(self.downloaded),
                format_size(self.total)
            )
        } else {
            format_size(self.downloaded)
        }
    }

    pub fn formatted_speed(&self) -> String {
        format_speed(self.speed_bps)
    }

    pub fn formatted_eta(&self) -> String {
        format_eta(self.eta_secs)
    }
}
