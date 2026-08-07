use iced::widget::{button, center, column, container, row, text};
use iced::{Element, Task as Command};
use iced_layershell::actions::LayerShellCustomActionWithId;
use iced_layershell::application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};

use crate::conference;
use crate::model::Meeting;

pub fn run(meeting: Meeting) -> Result<(), iced_layershell::Error> {
    application(
        move || App::new(meeting.clone()),
        App::namespace,
        App::update,
        App::view,
    )
    .settings(Settings {
        layer_settings: LayerShellSettings {
            layer: Layer::Overlay,
            anchor: Anchor::all(),
            size: None,
            exclusive_zone: -1,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

struct App {
    meeting: Meeting,
}

#[derive(Debug, Clone)]
enum Message {
    Join,
    Dismiss,
}

impl App {
    fn new(meeting: Meeting) -> (Self, Command<Message>) {
        (Self { meeting }, Command::none())
    }

    fn namespace() -> String {
        "tardy-notif".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Join => {
                if let Some(url) = conference::join_url(&self.meeting) {
                    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
                }
                std::process::exit(0);
            }
            Message::Dismiss => {
                std::process::exit(0);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let summary = self.meeting.summary.as_deref().unwrap_or("(untitled)");
        let url = conference::join_url(&self.meeting);

        let join_label = if url.is_none() { text("No link") } else { text("Join") };

        let join = button(center(join_label.size(22)).width(180).height(56))
            .on_press_maybe(url.map(move |_| Message::Join))
            .style(|_theme, _status| iced::widget::button::Style::default());

        let dismiss = button(center(text("Dismiss").size(22)).width(180).height(56))
            .on_press(Message::Dismiss)
            .style(|_theme, _status| iced::widget::button::Style::default());

        center(
            container(
                column![
                    text("Your meeting starts now").size(28),
                    text(summary).size(48),
                    row![join, dismiss]
                        .spacing(24)
                        .align_y(iced::Alignment::Center),
                ]
                .spacing(24)
                .align_x(iced::Alignment::Center),
            )
            .padding(40),
        )
        .into()
    }
}

impl TryInto<LayerShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<LayerShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}