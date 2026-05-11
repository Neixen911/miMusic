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
    PLAYING(PlayingInterface),
    DOWNLOADING(DownloadingInterface),
    SONGS(SongsInterface),
    PLAYLISTS(PlaylistsInterface)
}

#[derive(Debug, PartialEq)]
pub enum PlayingInterface {
    DEFAULT
}

#[derive(Debug, PartialEq)]
pub enum DownloadingInterface {
    DEFAULT
}

#[derive(Debug, PartialEq)]
pub enum SongsInterface {
    DEFAULT,
    ADD,
    MODIFY,
    DELETE
}

#[derive(Debug, PartialEq)]
pub enum PlaylistsInterface {
    DEFAULT,
    ADD,
    MODIFY,
    DELETE
}

pub struct Registry {
    services: HashMap<TypeId, Arc<Mutex<dyn Service + Send + Sync>>>,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            services: HashMap::new(),
        }
    }

    pub fn add_service<T>(&mut self, service: T)
    where
        T: Service + Send + Sync + 'static
    {
        self.services.insert(TypeId::of::<T>(), Arc::new(Mutex::new(service)));
    }

    pub fn get_service<T: 'static>(&self) -> Option<Arc<Mutex<dyn Service + Send + Sync>>> {
        let mut result = None;
        if self.services.get(&TypeId::of::<T>()).is_some() {
            result = self.services.get(&TypeId::of::<T>()).cloned()
        }

        result
    }
}

pub trait Service: Any + Send + Sync {
    fn new(service_name: ServiceName) -> Self
    where
        Self: Sized;
    fn as_any(&mut self) -> &mut dyn Any;
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