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
        start: parse_dt(event, "DTSTART"),
        end: parse_dt(event, "DTEND"),
        description,
        location,
        url,
        rrule: non_empty(event.property_value("RRULE")),
        all_day: is_all_day(event),
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

/// Date-valued DTSTART (`DTSTART;VALUE=DATE:20260807`) marks an all-day event.
/// Some backends drop the parameter, so fall back to the value's shape.
fn is_all_day(event: &icalendar::Event) -> bool {
    let date_valued = event
        .properties()
        .get("DTSTART")
        .and_then(|prop| prop.params().get("VALUE"))
        .is_some_and(|param| param.value().eq_ignore_ascii_case("DATE"));

    date_valued
        || event
            .property_value("DTSTART")
            .is_some_and(|value| NaiveDate::parse_from_str(value, "%Y%m%d").is_ok())
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

fn parse_dt(event: &icalendar::Event, name: &str) -> Option<DateTime<Utc>> {
    let value = event.property_value(name)?;

    // All-day event (no time component).
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
        let naive = date.and_hms_opt(0, 0, 0)?;
        let local = chrono::Local;
        return Some(local.from_local_datetime(&naive).single()?.with_timezone(&Utc));
    }

    let naive = parse_naive(value)?;
    let tz = property_tz(event, name);
    tz.from_local_datetime(&naive)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn test_parse_icalendar_tzid() {
        let ical = "BEGIN:VCALENDAR\n\
            BEGIN:VEVENT\n\
            UID:123\n\
            SUMMARY:Test Meeting\n\
            DTSTART;TZID=Europe/London:20260601T151500\n\
            DTEND;TZID=Europe/London:20260601T161500\n\
            END:VEVENT\n\
            END:VCALENDAR";

        let range_start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 6, 1, 23, 59, 59).unwrap();

        let meetings = parse_icalendar(ical, range_start, range_end);
        assert_eq!(meetings.len(), 1);
        let start = meetings[0].start.unwrap();
        // 15:15 in BST (UTC+1) is 14:15 UTC
        assert_eq!(start.hour(), 14);
        assert_eq!(start.minute(), 15);
    }

    fn parse(vevent: &str) -> Vec<Meeting> {
        let ical = format!("BEGIN:VCALENDAR\nVERSION:2.0\n{vevent}\nEND:VCALENDAR\n");
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 14, 0, 0, 0).unwrap();
        parse_icalendar(&ical, start, end)
    }

    #[test]
    fn date_valued_dtstart_is_all_day() {
        let meetings = parse(
            "BEGIN:VEVENT\nUID:holiday\nSUMMARY:Bank Holiday\nDTSTART;VALUE=DATE:20260807\nDTEND;VALUE=DATE:20260808\nEND:VEVENT",
        );

        assert_eq!(meetings.len(), 1);
        assert!(meetings[0].all_day);
    }

    #[test]
    fn bare_date_dtstart_is_all_day() {
        let meetings = parse(
            "BEGIN:VEVENT\nUID:holiday\nSUMMARY:Bank Holiday\nDTSTART:20260807\nDTEND:20260808\nEND:VEVENT",
        );

        assert_eq!(meetings.len(), 1);
        assert!(meetings[0].all_day);
    }

    #[test]
    fn timed_dtstart_is_not_all_day() {
        let meetings = parse(
            "BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup\nDTSTART:20260807T090000Z\nDTEND:20260807T091500Z\nEND:VEVENT",
        );

        assert_eq!(meetings.len(), 1);
        assert!(!meetings[0].all_day);
    }

    #[test]
    fn recurring_all_day_keeps_the_flag() {
        let meetings = parse(
            "BEGIN:VEVENT\nUID:birthday\nSUMMARY:Birthday\nDTSTART;VALUE=DATE:20260807\nDTEND;VALUE=DATE:20260808\nRRULE:FREQ=DAILY;COUNT=3\nEND:VEVENT",
        );

        assert!(!meetings.is_empty());
        assert!(meetings.iter().all(|m| m.all_day));
    }
}
