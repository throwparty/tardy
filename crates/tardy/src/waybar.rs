use chrono::{DateTime, Local, Utc};
use serde::Serialize;

use crate::model::Meeting;

#[derive(Debug, Serialize)]
struct Waybar {
    text: String,
    alt: &'static str,
    class: String,
    tooltip: String,
}

pub fn render(meetings: &[Meeting], now: DateTime<Utc>) -> String {
    let line = Waybar {
        text: text(meetings, now),
        alt: alt(meetings, now),
        class: class(meetings, now),
        tooltip: tooltip(meetings),
    };
    serde_json::to_string(&line).expect("waybar line must serialize")
}

/// The meeting the bar shows and `tardy open` joins. All-day events are
/// skipped: they would otherwise occupy the bar for the whole day.
pub fn visible(meetings: &[Meeting], now: DateTime<Utc>) -> Option<&Meeting> {
    meetings
        .iter()
        .filter(|m| !m.all_day && m.end.is_none_or(|e| e >= now))
        .min_by_key(|m| m.start)
}

fn text(meetings: &[Meeting], now: DateTime<Utc>) -> String {
    let Some(next) = visible(meetings, now) else {
        return String::new();
    };

    let summary = next.summary.as_deref().unwrap_or("(untitled)");
    match (next.start, next.end) {
        (Some(start), Some(end)) if start <= now && now <= end => {
            format!("{summary} · till {}", fmt(end))
        }
        (Some(start), _) => format!("{} · {summary}", fmt(start)),
        _ => summary.to_string(),
    }
}

fn alt(meetings: &[Meeting], now: DateTime<Utc>) -> &'static str {
    if meetings.iter().any(|m| m.is_ongoing(now)) {
        "ongoing"
    } else if visible(meetings, now).is_none() {
        "none"
    } else {
        "scheduled"
    }
}

fn class(meetings: &[Meeting], now: DateTime<Utc>) -> String {
    if meetings.iter().any(|m| m.is_ongoing(now)) {
        "live".to_string()
    } else if visible(meetings, now).is_none() {
        "empty".to_string()
    } else {
        "scheduled".to_string()
    }
}

fn tooltip(meetings: &[Meeting]) -> String {
    if meetings.is_empty() {
        return "No meetings".to_string();
    }

    let today = Local::now().date_naive();
    let mut lines = Vec::new();
    let mut current_day = None;

    for meeting in meetings {
        let day = meeting.start.map(|dt| dt.with_timezone(&Local).date_naive());
        if day != current_day {
            lines.push(match day {
                Some(day) => date_heading(day, today),
                None => "Unscheduled".to_string(),
            });
            current_day = day;
        }
        lines.push(format!("    {}", meeting_line(meeting)));
    }

    lines.join("\n")
}

fn meeting_line(meeting: &Meeting) -> String {
    let summary = meeting.summary.as_deref().unwrap_or("(untitled)");
    if meeting.all_day {
        return format!("All day · {summary}");
    }
    match (meeting.start, meeting.end) {
        (Some(start), Some(end)) => format!("{}–{} · {summary}", fmt(start), fmt(end)),
        (Some(start), None) => format!("{} · {summary}", fmt(start)),
        _ => summary.to_string(),
    }
}

fn date_heading(date: chrono::NaiveDate, today: chrono::NaiveDate) -> String {
    if date == today {
        "Today".to_string()
    } else if date == today + chrono::Days::new(1) {
        "Tomorrow".to_string()
    } else {
        date.format("%a %d %b").to_string()
    }
}

fn fmt(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 7, 9, 0, 0).unwrap()
    }

    fn meeting(summary: &str, start: DateTime<Utc>, minutes: i64, all_day: bool) -> Meeting {
        Meeting {
            uid: summary.to_string(),
            summary: Some(summary.to_string()),
            start: Some(start),
            end: Some(start + Duration::minutes(minutes)),
            description: None,
            location: None,
            url: None,
            rrule: None,
            all_day,
        }
    }

    fn holiday() -> Meeting {
        meeting("Bank Holiday", now() - Duration::hours(9), 24 * 60, true)
    }

    fn standup() -> Meeting {
        meeting("Standup", now() + Duration::minutes(30), 15, false)
    }

    #[test]
    fn all_day_stays_out_of_the_bar() {
        let meetings = vec![holiday(), standup()];

        let text = text(&meetings, now());
        assert!(text.contains("Standup"), "{text}");
        assert!(!text.contains("Bank Holiday"), "{text}");
        assert_eq!(class(&meetings, now()), "scheduled");
        assert_eq!(alt(&meetings, now()), "scheduled");
    }

    #[test]
    fn all_day_shows_in_the_tooltip() {
        let tooltip = tooltip(&[holiday(), standup()]);
        assert!(tooltip.contains("All day · Bank Holiday"), "{tooltip}");
        assert!(tooltip.contains("Standup"), "{tooltip}");
    }

    #[test]
    fn all_day_alone_leaves_the_bar_empty() {
        let meetings = vec![holiday()];

        assert_eq!(text(&meetings, now()), "");
        assert_eq!(class(&meetings, now()), "empty");
        assert_eq!(alt(&meetings, now()), "none");
        assert!(tooltip(&meetings).contains("All day · Bank Holiday"));
    }

    #[test]
    fn all_day_is_never_ongoing() {
        let meetings = vec![holiday()];

        assert!(!holiday().is_ongoing(now()));
        assert_ne!(class(&meetings, now()), "live");
    }

    #[test]
    fn visible_skips_all_day() {
        let meetings = vec![holiday(), standup()];
        let visible = visible(&meetings, now()).expect("a visible meeting");
        assert_eq!(visible.summary.as_deref(), Some("Standup"));
    }

    #[test]
    fn ongoing_timed_meeting_goes_live() {
        let retro = meeting("Retro", now() - Duration::minutes(5), 30, false);
        let meetings = vec![holiday(), retro];

        assert_eq!(class(&meetings, now()), "live");
        assert_eq!(alt(&meetings, now()), "ongoing");
    }
}