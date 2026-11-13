use std::path::Path;

use reqwest::Response;
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_TYPE};

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push(high * 16 + low);
            index += 3;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

#[doc(hidden)]
pub fn parse_content_disposition(value: &str) -> Option<String> {
    let mut plain_filename = None;

    for part in value.split(';') {
        let part = part.trim();

        if let Some(rest) = part.strip_prefix("filename*=") {
            let rest = rest.trim().trim_matches('"');
            let mut pieces = rest.splitn(3, '\'');
            let charset = pieces.next();
            let _language = pieces.next();
            let encoded = pieces.next();

            if charset.is_some() && encoded.is_some() {
                let decoded = percent_decode(encoded.unwrap_or_default());
                if !decoded.is_empty() {
                    return Some(decoded);
                }
            }
        } else if let Some(rest) = part.strip_prefix("filename=") {
            let filename = rest.trim().trim_matches('"');
            if !filename.is_empty() {
                plain_filename = Some(filename.to_string());
            }
        }
    }

    plain_filename
}

fn limit_filename(name: String) -> String {
    const MAX_FILENAME_BYTES: usize = 180;

    if name.len() <= MAX_FILENAME_BYTES {
        return name;
    }

    match Path::new(&name)
        .extension()
        .and_then(|value| value.to_str())
    {
        Some(extension) => {
            let suffix = format!(".{extension}");
            let base_length = MAX_FILENAME_BYTES.saturating_sub(suffix.len());
            let base: String = name.chars().take(base_length).collect();
            format!("{base}{suffix}")
        }
        None => name.chars().take(MAX_FILENAME_BYTES).collect(),
    }
}

#[doc(hidden)]
pub fn sanitize_filename(value: &str) -> Option<String> {
    let basename = value.rsplit(['/', '\\']).next().unwrap_or(value);
    let mut cleaned = String::with_capacity(basename.len());

    for character in basename.chars() {
        if character.is_control()
            || matches!(
                character,
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
            )
        {
            cleaned.push('_');
        } else {
            cleaned.push(character);
        }
    }

    let cleaned = cleaned
        .trim_matches(|character: char| character == ' ' || character == '.')
        .to_string();
    if cleaned.is_empty() {
        return None;
    }

    Some(limit_filename(cleaned))
}

#[doc(hidden)]
pub fn default_filename(content_type: Option<&str>) -> String {
    let extension = match content_type.unwrap_or_default() {
        "application/vnd.android.package-archive" => "apk",
        "application/zip" | "application/x-zip-compressed" => "zip",
        "application/pdf" => "pdf",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "audio/mpeg" => "mp3",
        "video/mp4" => "mp4",
        "text/plain" => "txt",
        _ => "bin",
    };

    format!("rget-download.{extension}")
}

pub(crate) fn response_filename(response: &Response, content_type: Option<&str>) -> String {
    let from_header = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_content_disposition)
        .and_then(|filename| sanitize_filename(&filename));
    if let Some(filename) = from_header {
        return filename;
    }

    let from_url = response
        .url()
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|segment| !segment.is_empty())
        .map(percent_decode)
        .and_then(|filename| sanitize_filename(&filename));
    if let Some(filename) = from_url {
        return filename;
    }

    default_filename(content_type)
}

pub(crate) fn response_content_type(response: &Response) -> Option<String> {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or(value)
                .trim()
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty())
}
