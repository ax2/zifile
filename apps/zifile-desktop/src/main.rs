use iced::widget::{button, column, container, row, rule, text};
use iced::{Element, Fill, Theme};
use zifile_core::ArchiveFormat;

pub fn main() -> iced::Result {
    iced::application(ZiFile::default, update, view)
        .title("ZiFile")
        .theme(theme)
        .window_size((1_100.0, 720.0))
        .antialiasing(true)
        .run()
}

#[derive(Debug)]
struct ZiFile {
    status: String,
}

impl Default for ZiFile {
    fn default() -> Self {
        Self {
            status: "Stage 0 foundation is ready".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    OpenArchive,
    CreateArchive,
    ShowFormats,
}

fn update(state: &mut ZiFile, message: Message) {
    state.status = match message {
        Message::OpenArchive => "Archive opening arrives in Stage 1 Alpha",
        Message::CreateArchive => "Archive creation arrives in Stage 1 Alpha",
        Message::ShowFormats => "Format registry loaded from zifile-core",
    }
    .to_owned();
}

fn theme(_state: &ZiFile) -> Theme {
    Theme::TokyoNight
}

fn view(state: &ZiFile) -> Element<'_, Message> {
    let actions = row![
        button("Open archive").on_press(Message::OpenArchive),
        button("Create archive").on_press(Message::CreateArchive),
        button("Format plan").on_press(Message::ShowFormats),
    ]
    .spacing(12);

    let format_count = ArchiveFormat::ALL.len();
    let content = column![
        text("ZiFile").size(42),
        text("Fast, safe archive tools for Windows").size(20),
        rule::horizontal(1),
        text("A clean Rust foundation for opening, creating, and safely extracting archives."),
        actions,
        text(format!(
            "{format_count} formats are represented in the current roadmap."
        )),
        text(&state.status).size(14),
    ]
    .spacing(20)
    .max_width(760);

    container(content)
        .width(Fill)
        .height(Fill)
        .padding(48)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}
