pub mod adapter;
pub mod composite_adapter;
pub mod halley;
pub mod handlers;

use std::fmt;

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

/// Pre-flight bound on how far back a manual ingest request may reach.
/// `ingest-core` has no article-level publish-date awareness (see Plan 0029's
/// Non-Goals) — this only bounds the *request*, it does not filter which
/// chunks within a scraped page get embedded.
const MAX_WINDOW_DAYS: i64 = 366;

#[derive(Debug, Clone, PartialEq)]
pub enum RecencyWindow {
    Days(u32),
    Month { year: i32, month: u32 },
}

#[derive(Debug, PartialEq)]
pub enum RecencyWindowError {
    InvalidFormat(String),
    TooOld(String),
}

impl fmt::Display for RecencyWindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecencyWindowError::InvalidFormat(input) => {
                write!(
                    f,
                    "invalid window '{input}': expected '<N>d' (e.g. '30d') or 'YYYY-MM' (e.g. '2026-07')"
                )
            }
            RecencyWindowError::TooOld(msg) => write!(f, "window too old: {msg}"),
        }
    }
}

impl std::error::Error for RecencyWindowError {}

impl RecencyWindow {
    pub fn parse(input: &str) -> Result<Self, RecencyWindowError> {
        Self::parse_at(input, chrono::Utc::now().date_naive())
    }

    fn parse_at(input: &str, reference: NaiveDate) -> Result<Self, RecencyWindowError> {
        if let Some(days_str) = input.strip_suffix('d') {
            let days: u32 = days_str
                .parse()
                .map_err(|_| RecencyWindowError::InvalidFormat(input.to_string()))?;
            if i64::from(days) > MAX_WINDOW_DAYS {
                return Err(RecencyWindowError::TooOld(format!(
                    "{days} days exceeds the {MAX_WINDOW_DAYS}-day maximum"
                )));
            }
            return Ok(RecencyWindow::Days(days));
        }

        if let Some((year_str, month_str)) = input.split_once('-')
            && year_str.len() == 4
        {
            let year: i32 = year_str
                .parse()
                .map_err(|_| RecencyWindowError::InvalidFormat(input.to_string()))?;
            let month: u32 = month_str
                .parse()
                .map_err(|_| RecencyWindowError::InvalidFormat(input.to_string()))?;
            if !(1..=12).contains(&month) {
                return Err(RecencyWindowError::InvalidFormat(input.to_string()));
            }

            let requested = NaiveDate::from_ymd_opt(year, month, 1)
                .ok_or_else(|| RecencyWindowError::InvalidFormat(input.to_string()))?;
            let reference_month_start =
                NaiveDate::from_ymd_opt(reference.year(), reference.month(), 1)
                    .expect("reference date's own year/month must be valid");
            if requested < reference_month_start
                && (reference_month_start - requested).num_days() > MAX_WINDOW_DAYS
            {
                return Err(RecencyWindowError::TooOld(format!(
                    "{year}-{month:02} is more than {MAX_WINDOW_DAYS} days before {reference}"
                )));
            }

            return Ok(RecencyWindow::Month { year, month });
        }

        Err(RecencyWindowError::InvalidFormat(input.to_string()))
    }

    /// The earliest date this window reaches back to, relative to `reference`.
    /// Used by the Halley curation path (Plan 0030) to bound pagination — a
    /// `Days(n)` window reaches back `n` days; a `Month{y,m}` window reaches
    /// back to that month's 1st, through `reference`.
    pub fn cutoff_date(&self, reference: NaiveDate) -> NaiveDate {
        match self {
            RecencyWindow::Days(days) => reference - chrono::Duration::days(i64::from(*days)),
            RecencyWindow::Month { year, month } => NaiveDate::from_ymd_opt(*year, *month, 1)
                .expect("RecencyWindow::Month always holds a valid year/month, checked at parse"),
        }
    }
}

impl fmt::Display for RecencyWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecencyWindow::Days(days) => write!(f, "{days}d"),
            RecencyWindow::Month { year, month } => write!(f, "{year}-{month:02}"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestManualRequest {
    pub section: String,
    pub src: String,
    pub window: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestManualResponse {
    pub section: String,
    pub src: String,
    pub window: String,
    pub status: String,
}

#[derive(Debug)]
pub enum IngestManualError {
    InvalidWindow(RecencyWindowError),
    RobotsTxt(String),
    Ingest(String),
}

impl fmt::Display for IngestManualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngestManualError::InvalidWindow(e) => write!(f, "{e}"),
            IngestManualError::RobotsTxt(msg) => write!(f, "robots.txt: {msg}"),
            IngestManualError::Ingest(msg) => write!(f, "ingest error: {msg}"),
        }
    }
}

impl std::error::Error for IngestManualError {}

#[async_trait]
pub trait IngestManualAdminPort: Send + Sync {
    async fn ingest(
        &self,
        section: &str,
        src: &str,
        window: RecencyWindow,
    ) -> Result<IngestManualResponse, IngestManualError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 27).expect("valid reference date")
    }

    #[test]
    fn should_parse_days_window_from_valid_string() {
        let result = RecencyWindow::parse_at("30d", reference());
        assert_eq!(result, Ok(RecencyWindow::Days(30)));
    }

    #[test]
    fn should_parse_month_window_from_valid_string() {
        let result = RecencyWindow::parse_at("2026-07", reference());
        assert_eq!(
            result,
            Ok(RecencyWindow::Month {
                year: 2026,
                month: 7
            })
        );
    }

    #[test]
    fn should_reject_completely_invalid_format() {
        let result = RecencyWindow::parse_at("banana", reference());
        assert_eq!(
            result,
            Err(RecencyWindowError::InvalidFormat("banana".to_string()))
        );
    }

    #[test]
    fn should_reject_days_window_exceeding_max() {
        let result = RecencyWindow::parse_at("400d", reference());
        assert!(matches!(result, Err(RecencyWindowError::TooOld(_))));
    }

    #[test]
    fn should_reject_month_window_older_than_max() {
        let result = RecencyWindow::parse_at("2020-01", reference());
        assert!(matches!(result, Err(RecencyWindowError::TooOld(_))));
    }

    #[test]
    fn should_reject_month_with_invalid_month_number() {
        let result = RecencyWindow::parse_at("2026-13", reference());
        assert!(matches!(result, Err(RecencyWindowError::InvalidFormat(_))));
    }

    #[test]
    fn should_compute_cutoff_date_for_days_window() {
        let cutoff = RecencyWindow::Days(30).cutoff_date(reference());
        assert_eq!(cutoff, NaiveDate::from_ymd_opt(2026, 6, 27).unwrap());
    }

    #[test]
    fn should_compute_cutoff_date_for_month_window_as_first_of_that_month() {
        let cutoff = RecencyWindow::Month {
            year: 2026,
            month: 3,
        }
        .cutoff_date(reference());
        assert_eq!(cutoff, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
    }

    #[test]
    fn should_format_days_window_display() {
        assert_eq!(RecencyWindow::Days(30).to_string(), "30d");
    }

    #[test]
    fn should_format_month_window_display() {
        assert_eq!(
            RecencyWindow::Month {
                year: 2026,
                month: 7
            }
            .to_string(),
            "2026-07"
        );
    }

    #[test]
    fn should_format_invalid_format_error_display() {
        let err = RecencyWindowError::InvalidFormat("banana".to_string());
        assert_eq!(
            err.to_string(),
            "invalid window 'banana': expected '<N>d' (e.g. '30d') or 'YYYY-MM' (e.g. '2026-07')"
        );
    }
}
