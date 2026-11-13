use reqwest::StatusCode;

use rget::{
    ContentRange, Progress, ResumeAction, ResumePlan, default_filename, format_eta, format_size,
    format_speed, parse_content_disposition, parse_content_range, plan_resume, sanitize_filename,
};

#[test]
fn parses_content_range_with_total() {
    assert_eq!(
        parse_content_range("bytes 100-199/1000"),
        Some(ContentRange {
            start: Some(100),
            end: Some(199),
            total: Some(1000),
        })
    );
}

#[test]
fn parses_unsatisfied_content_range() {
    assert_eq!(
        parse_content_range("bytes */1000"),
        Some(ContentRange {
            start: None,
            end: None,
            total: Some(1000),
        })
    );
}

#[test]
fn parses_unknown_total() {
    assert_eq!(
        parse_content_range("bytes 0-99/*"),
        Some(ContentRange {
            start: Some(0),
            end: Some(99),
            total: None,
        })
    );
}

#[test]
fn server_ignoring_range_restarts_partial_file() {
    let plan = plan_resume(100, StatusCode::OK, Some(1000), None);
    assert_eq!(
        plan,
        ResumePlan {
            action: ResumeAction::Restart,
            total: Some(1000),
        }
    );
}

#[test]
fn server_ignoring_range_with_complete_file_is_complete() {
    let plan = plan_resume(1000, StatusCode::OK, Some(1000), None);
    assert_eq!(
        plan,
        ResumePlan {
            action: ResumeAction::AlreadyComplete,
            total: Some(1000),
        }
    );
}

#[test]
fn partial_content_appends() {
    let range = ContentRange {
        start: Some(100),
        end: Some(999),
        total: Some(1000),
    };
    let plan = plan_resume(100, StatusCode::PARTIAL_CONTENT, None, Some(range));
    assert_eq!(
        plan,
        ResumePlan {
            action: ResumeAction::Append,
            total: Some(1000),
        }
    );
}

#[test]
fn range_not_satisfiable_with_complete_file_is_complete() {
    let range = ContentRange {
        start: None,
        end: None,
        total: Some(1000),
    };
    let plan = plan_resume(1000, StatusCode::RANGE_NOT_SATISFIABLE, None, Some(range));
    assert_eq!(
        plan,
        ResumePlan {
            action: ResumeAction::AlreadyComplete,
            total: Some(1000),
        }
    );
}

#[test]
fn mismatched_partial_range_is_rejected() {
    let range = ContentRange {
        start: Some(50),
        end: Some(999),
        total: Some(1000),
    };
    let plan = plan_resume(100, StatusCode::PARTIAL_CONTENT, None, Some(range));
    assert_eq!(plan.action, ResumeAction::Reject);
}

#[test]
fn parses_plain_content_disposition_filename() {
    assert_eq!(
        parse_content_disposition("attachment; filename=\"Example App.apk\""),
        Some("Example App.apk".to_string())
    );
}

#[test]
fn parses_encoded_content_disposition_filename() {
    assert_eq!(
        parse_content_disposition("attachment; filename*=UTF-8''Example%20App.apk"),
        Some("Example App.apk".to_string())
    );
}

#[test]
fn sanitizes_without_guessing_an_extension() {
    assert_eq!(
        sanitize_filename("archive.tar.gz"),
        Some("archive.tar.gz".to_string())
    );
    assert_eq!(
        sanitize_filename("no-extension"),
        Some("no-extension".to_string())
    );
    assert_eq!(
        sanitize_filename("../../unsafe name.apk"),
        Some("unsafe name.apk".to_string())
    );
}

#[test]
fn default_filename_uses_content_type() {
    assert_eq!(
        default_filename(Some("application/vnd.android.package-archive")),
        "rget-download.apk"
    );
    assert_eq!(
        default_filename(Some("application/zip")),
        "rget-download.zip"
    );
    assert_eq!(default_filename(None), "rget-download.bin");
}

#[test]
fn formats_sizes_speeds_and_etas_consistently() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1536), "1.5 KB");
    assert_eq!(format_size(100 * 1024), "100 KB");
    assert_eq!(format_size(1024 * 1024), "1.0 MB");

    assert_eq!(format_speed(0.0), "—");
    assert_eq!(format_speed(1536.0), "1.5 KB/s");

    assert_eq!(format_eta(None), "—");
    assert_eq!(format_eta(Some(5.0)), "5s");
    assert_eq!(format_eta(Some(303.0)), "5m 03s");
    assert_eq!(format_eta(Some(3900.0)), "1h 05m");

    let progress = Progress {
        downloaded: 1536,
        total: 2048,
        speed_bps: 1024.0,
        eta_secs: Some(1.0),
    };
    assert_eq!(progress.formatted_size(), "1.5 KB / 2.0 KB");
    assert_eq!(progress.formatted_speed(), "1.0 KB/s");
    assert_eq!(progress.formatted_eta(), "1s");
}
