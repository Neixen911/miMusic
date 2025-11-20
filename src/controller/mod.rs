use ratatui::{
    layout::Rect,
    Frame
};

pub trait Controller {
    fn render(&self, frame: &mut Frame, area: Rect);
}

pub mod playing;
pub mod downloading;

pub use playing::PlayingController;
pub use downloading::DownloadingController;
