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
    /// Date-valued event (holiday, out-of-office, birthday). Listed in the
    /// tooltip only: it never claims the bar and never raises the takeover.
    pub all_day: bool,
}

impl Meeting {
    /// An all-day event covers the whole day, so treating it as ongoing would
    /// pin the bar to "live" from midnight and fire a takeover nobody can join.
    pub fn is_ongoing(&self, now: DateTime<Utc>) -> bool {
        let Some((start, end)) = self.start.zip(self.end) else {
            return false;
        };
        !self.all_day && start <= now && now <= end
    }
}
