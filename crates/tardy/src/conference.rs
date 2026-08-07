use crate::model::Meeting;

const CONFERENCE_HOSTS: &[&str] = &[
    "meet.google.com",
    "zoom.us",
    "teams.microsoft.com",
    "webex.com",
];

pub fn join_url(meeting: &Meeting) -> Option<String> {
    if let Some(url) = &meeting.url {
        if !url.trim().is_empty() {
            return Some(url.trim().to_string());
        }
    }

    [&meeting.description, &meeting.location]
        .into_iter()
        .flatten()
        .find_map(|text| conference_url_in(text))
}

fn conference_url_in(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        if let Some(rest) = token
            .trim_end_matches(['.', ',', ')', ']', '>'])
            .strip_prefix("https://")
        {
            let host = rest.split(['/', ':']).next().unwrap_or_default();
            if CONFERENCE_HOSTS
                .iter()
                .any(|known| host == *known || host.ends_with(&format!(".{known}")))
            {
                return Some(format!("https://{rest}"));
            }
        }
    }
    None
}