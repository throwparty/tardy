mod conference;
mod eds;
mod event_parser;
mod model;
mod notify;
mod takeover;
mod waybar;

use std::error::Error as StdError;

use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};

#[derive(Debug)]
pub struct Error(Box<dyn StdError + Send + Sync>);

impl<E: StdError + Send + Sync + 'static> From<E> for Error {
    fn from(err: E) -> Self {
        Error(Box::new(err))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Parser)]
#[command(name = "tardy", about = "Never be late to meetings again")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Poll for live meetings and pop the fullscreen overlay when one starts
    Daemon {
        /// Seconds between polls (default 30)
        #[arg(long, default_value_t = 30)]
        interval: u64,
        /// Emit status line on stdout in FORMAT
        #[arg(long, value_enum, default_value_t = DaemonFormat::None)]
        format: DaemonFormat,
    },
    /// List meetings from the calendars available on this machine
    List {
        /// How many days ahead to look (default 5)
        #[arg(long)]
        days: Option<i64>,
    },
    /// Open the currently shown meeting in your browser
    Open,
    /// Exercises tardy's alerting surfaces with a demo meeting
    Test {
        #[command(subcommand)]
        runner: TestRunner,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Default, PartialEq, Eq)]
enum DaemonFormat {
    #[default]
    None,
    Ashell,
    Waybar,
}

#[derive(Subcommand)]
enum TestRunner {
    /// Show the fullscreen meeting-start overlay with a demo meeting
    Takeover,
    /// Send the desktop notification reminder with a demo meeting
    Notify,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::Daemon { interval, format } => {
            let connection = zbus::Connection::session().await?;
            let sources = eds::enumerate_calendars(&connection).await?;
            daemon(&connection, &sources, interval, format).await;
        }
        Command::List { days } => {
            let connection = zbus::Connection::session().await?;
            let sources = eds::enumerate_calendars(&connection).await?;
            let days = days.unwrap_or(5);
            let meetings = fetch_meetings(&connection, &sources, days).await;
            println!("{}", serde_json::to_string_pretty(&meetings)?);
        }
        Command::Open => {
            let connection = zbus::Connection::session().await?;
            let sources = eds::enumerate_calendars(&connection).await?;
            let meetings = fetch_meetings(&connection, &sources, 5).await;
            match waybar::visible(&meetings, Utc::now()) {
                Some(meeting) => open_meeting(meeting)?,
                None => eprintln!("[tardy] no meeting to open"),
            }
        }
        Command::Test { runner } => match runner {
            TestRunner::Takeover => {
                takeover::run(demo_meeting())?;
            }
            TestRunner::Notify => {
                notify::send(demo_meeting()).await?;
            }
        },
    }

    Ok(())
}

async fn daemon(
    connection: &zbus::Connection,
    sources: &[eds::CalendarSource],
    interval_secs: u64,
    format: DaemonFormat,
) {
    let mut recorded: Vec<String> = Vec::new();
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval_secs.min(30)));
    loop {
        timer.tick().await;
        let meetings = fetch_meetings(connection, sources, 5).await;
        let now = Utc::now();

        if matches!(format, DaemonFormat::Ashell | DaemonFormat::Waybar) {
            println!("{}", waybar::render(&meetings, now));
        }

        for meeting in meetings.iter().filter(|m| is_live(m, now)) {
            if recorded.contains(&dedupe_key(&meeting.uid)) {
                continue;
            }
            recorded.push(dedupe_key(&meeting.uid));
            let meeting = meeting.clone();
            std::thread::spawn(move || {
                if let Err(e) = takeover::run(meeting) {
                    eprintln!("[tardy] takeover failed: {e}");
                }
            });
        }
    }
}

/// Recurring-expanded EDS objects share a base UID and differ only in the
/// `_R<instance>` suffix, so strip that off for dedup purposes.
fn dedupe_key(uid: &str) -> String {
    match uid.find("_R") {
        Some(idx) => uid[..idx].to_string(),
        None => uid.to_string(),
    }
}

fn demo_meeting() -> model::Meeting {
    model::Meeting {
        uid: String::new(),
        summary: Some("Demo Meeting".to_string()),
        start: Some(Utc::now()),
        end: Some(Utc::now() + Duration::minutes(30)),
        description: None,
        location: None,
        url: Some("https://meet.google.com/abc-defg-hij".to_string()),
        rrule: None,
    }
}

fn open_meeting(meeting: &model::Meeting) -> Result<(), Error> {
    let Some(url) = meeting.url.as_deref().or(meeting.location.as_deref()) else {
        let summary = meeting.summary.as_deref().unwrap_or("(untitled)");
        eprintln!("[tardy] '{summary}' has no URL to open");
        return Ok(());
    };
    std::process::Command::new("xdg-open").arg(url).spawn()?;
    Ok(())
}

fn is_live(meeting: &model::Meeting, now: chrono::DateTime<Utc>) -> bool {
    matches!((meeting.start, meeting.end), (Some(s), Some(e)) if s <= now && now <= e)
}

async fn fetch_meetings(
    connection: &zbus::Connection,
    sources: &[eds::CalendarSource],
    days: i64,
) -> Vec<model::Meeting> {
    let now = Utc::now();
    let start = now - Duration::hours(1);
    let end = now + Duration::days(days);

    let mut all = Vec::new();
    for source in sources {
        match eds::query_events(connection, source, start, end).await {
            Ok(mut meetings) => all.append(&mut meetings),
            Err(e) => eprintln!("[tardy] {}: {e}", source.uid),
        }
    }

    all.retain(|m| in_window(m, start, end));
    all.sort_by_key(|a| a.start);
    all.dedup_by(|a, b| a.uid == b.uid && a.start == b.start);
    all
}

fn in_window(
    meeting: &model::Meeting,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
) -> bool {
    match (meeting.start, meeting.end) {
        (None, None) => false,
        (Some(s), None) => s >= start && s <= end,
        (None, Some(e)) => e >= start && e <= end,
        (Some(s), Some(e)) => s <= end && e >= start,
    }
}
