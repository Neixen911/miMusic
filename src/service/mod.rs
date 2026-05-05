use ratatui::{
    layout::Rect,
    Frame
};
use std::sync::Arc;

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

pub struct Registry {
    services: Vec<Arc<dyn Service>>,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            services: Vec::new(),
        }
    }

    pub fn add_service(&mut self, service: Arc<dyn Service>) {
        self.services.push(service);
    }

    pub fn get_service(&self, service_name: ServiceName) -> Option<Arc<dyn Service>> {
        let mut asked_service = self.services.clone();
        asked_service.retain(|service| service_name == *service.get_name());
        let mut result = None;
        if !asked_service.is_empty() {
            result = Some(asked_service[0].clone())
        }

        result
    }
}

pub trait Service {
    fn new(service_name: ServiceName) -> Self
    where
        Self: Sized;
    fn get_name(&self) -> &ServiceName;
    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName, registry: &Registry)
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