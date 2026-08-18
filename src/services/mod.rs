use ratatui::{
    layout::Rect,
    Frame
};

#[derive(PartialEq)]
pub enum ServiceName {
    NONE,
    PLAYER,
    DOWNLOAD,
    SONGS,
    PLAYLISTS
}

pub trait Service {
    fn new(service_name: ServiceName) -> Self
    where
        Self: Sized;
    fn get_name(&self) -> &ServiceName;
    fn update(&mut self)
    where
        Self: Sized;
    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName)
    where
        Self: Sized;
}

pub mod player;
pub mod download;
pub mod playlists;
pub mod songs;

pub use player::PlayerService;
pub use download::DownloadService;
pub use playlists::PlaylistsService;
pub use songs::SongsService;