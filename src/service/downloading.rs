use super::Service;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Gauge},
    Frame
};

pub struct DownloadingService {
    pub input_downloading: String,
    pub state_download: f64
}

impl DownloadingService {
    pub fn new() -> Self {
        DownloadingService {
            input_downloading: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
            state_download: 0.0
        }
    }
}

impl Service for DownloadingService {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);
        
        //let download_border_style = if is_active {Color::Magenta} else {Color::Reset};
        let download_border_style = Color::Reset;
        let downloading_section = Block::default()
            .title(Line::from("Download URL"))
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(download_border_style);
        frame.render_widget(downloading_section, area);

        let downloading_label = Span::styled(
            &self.input_downloading,
            Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC),
        );
        let downloading_gauge_section = Gauge::default()
            .ratio(self.state_download)
            .gauge_style(Color::Magenta)
            .label(downloading_label);
        frame.render_widget(downloading_gauge_section, chunks[0]);
    }
}
