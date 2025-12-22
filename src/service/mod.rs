use ratatui::{
    layout::Rect,
    Frame
};

pub trait Service {
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

pub mod playing;
pub mod downloading;
pub mod playlists;
pub mod songs;

pub use playing::PlayingService;
pub use downloading::DownloadingService;
pub use playlists::PlaylistsService;
pub use songs::SongsService;