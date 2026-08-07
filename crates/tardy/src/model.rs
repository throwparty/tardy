use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Meeting {
    pub uid: String,
    pub summary: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub rrule: Option<String>,
}
