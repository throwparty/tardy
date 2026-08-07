use std::str::FromStr;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz as NamedTz;
use icalendar::Calendar;
use icalendar::Component;
use rrule::{RRule, RRuleSet, Tz};

use crate::model::Meeting;

pub fn parse_icalendar(
    ical: &str,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> Vec<Meeting> {
    let cal = Calendar::from_str(ical).ok();
    let Some(component) = cal.and_then(|cal| cal.components.first().cloned()) else {
        return Vec::new();
    };
    let Some(event) = component.as_event() else {
        return Vec::new();
    };

    let uid = event.property_value("UID").unwrap_or_default().to_string();
    let summary = non_empty(event.property_value("SUMMARY"));
    let location = non_empty(event.property_value("LOCATION"));
    let description = non_empty(event.property_value("DESCRIPTION"));
    let url = extract_url(description.as_deref(), event);

    let base = Meeting {
        uid,
        summary,
        start: parse_dt(event.property_value("DTSTART")),
        end: parse_dt(event.property_value("DTEND")),
        description,
        location,
        url,
        rrule: non_empty(event.property_value("RRULE")),
    };

    let Some(rrule) = base.rrule.clone() else {
        return vec![base];
    };

    expand_recurrence(base, event, &rrule, range_start, range_end)
}

fn expand_recurrence(
    meeting: Meeting,
    event: &icalendar::Event,
    rrule: &str,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> Vec<Meeting> {
    let Some(dtstart_raw) = event.property_value("DTSTART") else {
        return Vec::new();
    };
    let Some(dtstart_naive) = parse_naive(dtstart_raw) else {
        return Vec::new();
    };
    let tz = property_tz(event, "DTSTART");
    let Some(dt_start) = tz.from_local_datetime(&dtstart_naive).earliest() else {
        return Vec::new();
    };

    let Ok(rrule_parsed) = RRule::from_str(rrule) else {
        return vec![meeting];
    };
    let Ok(rrule) = rrule_parsed.validate(dt_start) else {
        return vec![meeting];
    };

    let duration = event
        .property_value("DTEND")
        .and_then(parse_naive)
        .and_then(|naive| tz.from_local_datetime(&naive).earliest())
        .map(|dt_end| dt_end - dt_start)
        .unwrap_or_else(|| chrono::Duration::hours(1));

    let mut occurrences = RRuleSet::new(dt_start).rrule(rrule);
    if let Some(exdates) = event.property_value("EXDATE") {
        for raw in exdates.split(',') {
            if let Some(naive) = parse_naive(raw) {
                if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
                    occurrences = occurrences.exdate(dt);
                }
            }
        }
    }

    let mut meetings = Vec::new();
    for occurrence in &occurrences {
        let occurrence = occurrence.with_timezone(&Utc);
        if occurrence > range_end {
            break;
        }
        if occurrence < range_start {
            continue;
        }

        let mut expanded = meeting.clone();
        expanded.start = Some(occurrence);
        expanded.end = Some(occurrence + duration);
        expanded.rrule = None;
        meetings.push(expanded);
    }
    meetings
}

fn property_tz(event: &icalendar::Event, name: &str) -> Tz {
    let tzid = event
        .properties()
        .get(name)
        .and_then(|prop| prop.params().get("TZID"))
        .map(|param| param.value());

    if let Some(tzid) = tzid.and_then(|tzid| NamedTz::from_str(tzid).ok()) {
        return Tz::from(tzid);
    }
    if event.property_value(name).is_some_and(|v| v.ends_with('Z')) {
        return Tz::from(Utc);
    }
    Tz::from(Local)
}

fn parse_naive(value: &str) -> Option<NaiveDateTime> {
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
        return date.and_hms_opt(0, 0, 0);
    }
    NaiveDateTime::parse_from_str(value.trim_end_matches('Z'), "%Y%m%dT%H%M%S").ok()
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value.and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    })
}

fn extract_url(description: Option<&str>, event: &icalendar::Event) -> Option<String> {
    non_empty(event.property_value("X-GOOGLE-CONFERENCE"))
        .or_else(|| non_empty(event.property_value("URL")))
        .or_else(|| extract_described_url(description))
}

fn extract_described_url(description: Option<&str>) -> Option<String> {
    let mut fallback = None;
    for line in description?.lines() {
        let line = line.trim();
        for token in line.split_whitespace() {
            let url = token.trim_end_matches([')', ',', '>', '"']);
            if let Some(path) = url.strip_prefix("https://") {
                let full = format!("https://{path}");
                if path.contains("meet.google.com") {
                    return Some(full);
                }
                fallback.get_or_insert(full);
            }
        }
    }
    fallback
}

fn parse_dt(value: Option<&str>) -> Option<DateTime<Utc>> {
    let value = value?;

    // All-day event (no time component).
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
        let naive = date.and_hms_opt(0, 0, 0)?;
        let local = chrono::Local;
        return Some(local.from_local_datetime(&naive).single()?.with_timezone(&Utc));
    }

    // UTC-suffixed timestamp.
    if let Some(stripped) = value.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S").ok()?;
        return Some(Utc.from_utc_datetime(&naive));
    }

    // Floating local time, treat as local.
    let naive = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;
    let local = chrono::Local;
    Some(local.from_local_datetime(&naive).single()?.with_timezone(&Utc))
}