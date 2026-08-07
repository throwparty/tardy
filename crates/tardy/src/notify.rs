use zbus::Proxy;

use crate::conference;
use crate::model::Meeting;

const NOTIFICATION_DEST: &str = "org.freedesktop.Notifications";
const NOTIFICATION_PATH: &str = "/org/freedesktop/Notifications";
const NOTIFICATION_IFACE: &str = "org.freedesktop.Notifications";

pub async fn send(meeting: Meeting) -> Result<(), zbus::Error> {
    let connection = zbus::Connection::session().await?;
    let proxy =
        Proxy::new(&connection, NOTIFICATION_DEST, NOTIFICATION_PATH, NOTIFICATION_IFACE).await?;

    let summary = meeting.summary.as_deref().unwrap_or("(untitled)");

    let url = conference::join_url(&meeting);
    let body = match (meeting.start, url.as_deref()) {
        (Some(start), Some(url)) => format!("Starts at {} · {url}", start.format("%H:%M")),
        (Some(start), None) => format!("Starts at {}", start.format("%H:%M")),
        (None, Some(url)) => url.to_string(),
        (None, None) => "Meeting starting soon".to_string(),
    };

    let _notif_id: u32 = proxy
        .call_method(
            "Notify",
            &(
                "tardy",
                0u32,
                "dialog-information",
                &format!("Meeting: {summary}"),
                body.as_str(),
                Vec::<&str>::new(),
                std::collections::HashMap::<String, zbus::zvariant::Value>::new(),
                -1i32,
            ),
        )
        .await?
        .body()
        .deserialize()?;

    Ok(())
}