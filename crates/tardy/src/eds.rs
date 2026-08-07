use zbus::fdo::{ManagedObjects, ObjectManagerProxy};
use zbus::{Connection, Proxy};

use crate::model::Meeting;

pub const SOURCES_DEST: &str = "org.gnome.evolution.dataserver.Sources5";
pub const SOURCES_PATH: &str = "/org/gnome/evolution/dataserver/SourceManager";
pub const SOURCE_IFACE: &str = "org.gnome.evolution.dataserver.Source";
pub const CALENDAR_FACTORY_DEST: &str = "org.gnome.evolution.dataserver.Calendar8";
pub const CALENDAR_FACTORY_PATH: &str = "/org/gnome/evolution/dataserver/CalendarFactory";
pub const CALENDAR_FACTORY_IFACE: &str = "org.gnome.evolution.dataserver.CalendarFactory";
pub const CALENDAR_IFACE: &str = "org.gnome.evolution.dataserver.Calendar";

#[derive(Debug, Clone)]
pub struct CalendarSource {
    pub uid: String,
}

pub async fn enumerate_calendars(
    connection: &Connection,
) -> Result<Vec<CalendarSource>, crate::Error> {
    let manager = ObjectManagerProxy::builder(connection)
        .destination(SOURCES_DEST)?
        .path(SOURCES_PATH)?
        .build()
        .await?;

    let objects: ManagedObjects = manager.get_managed_objects().await?;

    let mut sources = Vec::new();
    for (path, _) in objects {
        let proxy = Proxy::new(connection, SOURCES_DEST, path, SOURCE_IFACE).await?;
        let data: String = proxy.get_property("Data").await?;
        let uid: String = proxy.get_property("UID").await?;
        let status: String = proxy.get_property("ConnectionStatus").await?;

        if status == "awaiting-credentials" {
            continue;
        }

        if is_calendar_source(&data) {
            sources.push(CalendarSource { uid });
        }
    }

    Ok(sources)
}

fn is_calendar_source(data: &str) -> bool {
    let mut in_calendar = false;

    for line in data.lines() {
        let line = line.trim_end();
        if line.starts_with('[') {
            in_calendar = line.starts_with("[Calendar]");
            continue;
        }
        if in_calendar && line.starts_with("BackendName") {
            return true;
        }
    }

    false
}

pub async fn query_events(
    connection: &Connection,
    source: &CalendarSource,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<Meeting>, crate::Error> {
    let factory = Proxy::new(
        connection,
        CALENDAR_FACTORY_DEST,
        CALENDAR_FACTORY_PATH,
        CALENDAR_FACTORY_IFACE,
    )
    .await?;

    let response = factory
        .call_method("OpenCalendar", &(source.uid.as_str(),))
        .await?;
    let (calendar_path, bus_name): (String, String) = response.body().deserialize()?;

    let calendar = Proxy::new(connection, bus_name.as_str(), calendar_path.as_str(), CALENDAR_IFACE)
        .await?;

    calendar.call_method("Open", &()).await?;

    let sexp = format!(
        "(occur-in-time-range? (make-time \"{}\") (make-time \"{}\"))",
        start.format("%Y%m%dT%H%M%SZ"),
        end.format("%Y%m%dT%H%M%SZ")
    );

    let response = calendar.call_method("GetObjectList", &(sexp,)).await?;
    let ical_objects: Vec<String> = response.body().deserialize()?;

    let mut meetings = Vec::new();
    for ical in ical_objects {
        meetings.extend(crate::event_parser::parse_icalendar(&ical, start, end));
    }

    Ok(meetings)
}
