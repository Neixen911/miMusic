use ratatui::{
    layout::Rect,
    Frame
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq)]
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
    fn as_any(&mut self) -> &mut dyn Any;
    fn get_name(&self) -> &ServiceName;
    fn update(&mut self)
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