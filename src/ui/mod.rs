use ratatui::{
    crossterm::event::KeyEvent,
    layout::Rect,
    Frame
};

#[derive(Debug, PartialEq)]
pub enum ServiceName {
    PLAYER,
    DOWNLOAD,
    SONGS,
    PLAYLISTS
}

#[derive(Debug, PartialEq)]
pub enum PopupState {
    NONE,
    ADD,
    MODIFY,
    DELETE
}

pub trait Service {
    fn new(service_name: ServiceName) -> Self
    where
        Self: Sized;
    fn get_name(&self) -> &ServiceName;
    fn handle_popup_events(&mut self, key_event: KeyEvent, mode: &PopupState)
    where
        Self: Sized;
    fn handle_events(&mut self, key_event: KeyEvent)
    where
        Self: Sized;
    fn get_hotkeys(&mut self, mode: &PopupState) -> String
    where
        Self: Sized;
    fn update(&mut self)
    where
        Self: Sized;
    fn render_popups(&mut self, frame: &mut Frame, mode: &PopupState)
    where
        Self: Sized;
    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName)
    where
        Self: Sized;
}

pub mod player_ui;
pub mod download_ui;
pub mod playlists_ui;
pub mod songs_ui;

pub use player_ui::PlayerService;
pub use download_ui::DownloadService;
pub use playlists_ui::PlaylistsService;
pub use songs_ui::SongsService;