use ratatui::{
    layout::Rect,
    Frame
};

#[derive(PartialEq)]
pub enum ServiceName {
    NONE,
    PLAYING(PlayingInterface),
    DOWNLOADING(DownloadingInterface),
    SONGS(SongsInterface),
    PLAYLISTS(PlaylistsInterface)
}

#[derive(PartialEq)]
pub enum PlayingInterface {
    DEFAULT
}

#[derive(PartialEq)]
pub enum DownloadingInterface {
    DEFAULT
}

#[derive(PartialEq)]
pub enum SongsInterface {
    DEFAULT,
    ADD,
    MODIFY,
    DELETE
}

#[derive(PartialEq)]
pub enum PlaylistsInterface {
    DEFAULT,
    ADD,
    MODIFY,
    DELETE
}

#[derive(PartialEq)]
pub enum ServiceName {
    NONE,
    PLAYING(PlayingInterface),
    DOWNLOADING(DownloadingInterface),
    SONGS(SongsInterface),
    PLAYLISTS(PlaylistsInterface)
}

#[derive(PartialEq)]
pub enum PlayingInterface {
    DEFAULT
}

#[derive(PartialEq)]
pub enum DownloadingInterface {
    DEFAULT
}

#[derive(PartialEq)]
pub enum SongsInterface {
    DEFAULT,
    ADD,
    MODIFY,
    DELETE
}

#[derive(PartialEq)]
pub enum PlaylistsInterface {
    DEFAULT,
    ADD,
    MODIFY,
    DELETE
}

pub trait Service {
    fn new(service_name: ServiceName) -> Self
    where
        Self: Sized;
    fn get_name(&self) -> &ServiceName;
    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName)
    where
        Self: Sized;
}

pub mod playing;
pub mod downloading;
pub mod playlists;
pub mod songs;

pub use playing::PlayingService;
pub use downloading::DownloadingService;
pub use playlists::PlaylistsService;
pub use songs::SongsService;