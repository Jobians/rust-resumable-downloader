use reqwest::header::{CONTENT_LENGTH, CONTENT_RANGE};
use reqwest::{Response, StatusCode};

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    pub start: Option<u64>,
    pub end: Option<u64>,
    pub total: Option<u64>,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeAction {
    Append,
    Restart,
    RetryWithoutRange,
    AlreadyComplete,
    Reject,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResumePlan {
    pub action: ResumeAction,
    pub total: Option<u64>,
}

#[doc(hidden)]
pub fn parse_content_range(value: &str) -> Option<ContentRange> {
    let value = value.trim();
    let rest = value.strip_prefix("bytes")?;
    let rest = rest.trim_start_matches([' ', '=']);
    let (range, total) = rest.split_once('/')?;

    let total = match total.trim() {
        "*" => None,
        value => Some(value.parse().ok()?),
    };

    let range = range.trim();
    if range == "*" {
        return Some(ContentRange {
            start: None,
            end: None,
            total,
        });
    }

    let (start, end) = range.split_once('-')?;
    let start = start.trim().parse().ok()?;
    let end = match end.trim() {
        "" => None,
        value => Some(value.parse().ok()?),
    };

    Some(ContentRange {
        start: Some(start),
        end,
        total,
    })
}

#[doc(hidden)]
pub fn plan_resume(
    start: u64,
    status: StatusCode,
    content_length: Option<u64>,
    content_range: Option<ContentRange>,
) -> ResumePlan {
    if status == StatusCode::PARTIAL_CONTENT {
        if let Some(range_start) = content_range.and_then(|range| range.start)
            && range_start != start
        {
            return ResumePlan {
                action: ResumeAction::Reject,
                total: None,
            };
        }

        let total = content_range
            .and_then(|range| range.total)
            .or_else(|| content_length.map(|length| length + start));
        let action = match total {
            Some(total) if start >= total => ResumeAction::AlreadyComplete,
            _ => ResumeAction::Append,
        };

        return ResumePlan { action, total };
    }

    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        let total = content_range.and_then(|range| range.total);
        let action = match total {
            Some(total) if start >= total => ResumeAction::AlreadyComplete,
            _ if start == 0 => ResumeAction::Reject,
            _ => ResumeAction::RetryWithoutRange,
        };

        return ResumePlan { action, total };
    }

    if status.is_success() {
        let total = content_length;
        let action = if start > 0 {
            match total {
                Some(total) if total == start => ResumeAction::AlreadyComplete,
                _ => ResumeAction::Restart,
            }
        } else {
            ResumeAction::Restart
        };

        return ResumePlan { action, total };
    }

    ResumePlan {
        action: ResumeAction::Reject,
        total: None,
    }
}

pub(crate) fn response_content_length(response: &Response) -> Option<u64> {
    response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

pub(crate) fn response_content_range(response: &Response) -> Option<ContentRange> {
    response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_content_range)
}
