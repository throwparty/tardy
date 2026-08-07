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

pub fn visible(meetings: &[Meeting], now: DateTime<Utc>) -> Option<&Meeting> {
    meetings
        .iter()
        .filter(|m| m.end.is_none_or(|e| e >= now))
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
    if meetings.iter().any(|m| is_ongoing(m, now)) {
        "ongoing"
    } else if meetings.is_empty() {
        "none"
    } else {
        "scheduled"
    }
}

fn class(meetings: &[Meeting], now: DateTime<Utc>) -> String {
    if meetings.iter().any(|m| is_ongoing(m, now)) {
        "live".to_string()
    } else if meetings.is_empty() {
        "empty".to_string()
    } else {
        "scheduled".to_string()
    }
}

fn is_ongoing(m: &Meeting, now: DateTime<Utc>) -> bool {
    matches!((m.start, m.end), (Some(s), Some(e)) if s <= now && now <= e)
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