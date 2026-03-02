use super::{Service, ServiceName};
use crate::tool::InputTool;

use ratatui::{
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Gauge},
    Frame
};

pub struct DownloadingService {
    pub service_name: ServiceName,
    pub input_downloading: InputTool,
    pub state_download: f64
}

impl DownloadingService {
    pub fn set_input_downloading(&mut self, new_input: String) {
        self.input_downloading.set_input(new_input);
    }

    pub fn get_input_downloading(&mut self) -> String {
        self.input_downloading.get_input()
    }

    pub fn get_input_position(&mut self) -> usize {
        self.input_downloading.get_position()
    }
}

impl Service for DownloadingService {
    fn new(service_name: ServiceName) -> Self {
        DownloadingService {
            service_name: service_name,
            input_downloading: InputTool::new("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()),
            state_download: 0.0
        }
    }

    fn get_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);
        let is_active = if active_service == self.get_name() {true} else {false};
        
        let download_border_style = if is_active {Color::Magenta} else {Color::Reset};
        let downloading_section = Block::default()
            .title(Line::from("Audio processing"))
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(download_border_style);
        frame.render_widget(downloading_section, area);

        let input = self.get_input_downloading();
        let downloading_label = Span::styled(
            &input,
            Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC),
        );
        let downloading_gauge_section = Gauge::default()
            .ratio(self.state_download)
            .gauge_style(Color::Magenta)
            .label(downloading_label);
        frame.render_widget(downloading_gauge_section, chunks[0]);

        // Render cursor only if service is the active one
        if is_active {
            frame.set_cursor_position(Position::new(
                // Right position - Left position to get the most inputable value
                // Adding the length of the input
                // Divide the all by 2 because the input set to the center
                // Adding 4 because of the 2 border + the last character + after it
                ((chunks[0].right() - chunks[0].left()) + self.get_input_downloading().len() as u16) / 2 + 4 - self.get_input_position() as u16,
                chunks[0].top()
            ));
        }
    }
}
