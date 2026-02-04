use super::{Service, ServiceName};

use ratatui::{
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Gauge},
    Frame
};

pub struct DownloadingService {
    pub service_name: ServiceName,
    pub input_downloading: String,
    pub state_download: f64,
    pub input_position: u16
}

impl DownloadingService {
    pub fn add_char_to_input(&mut self, new_char: char) {
        self.input_downloading.insert(self.input_downloading.len() - self.input_position as usize, new_char);
    }

    pub fn remove_previous_char_from_input(&mut self) {
        if self.input_downloading.len() - self.input_position as usize >= 1 {
            self.input_downloading.remove(self.input_downloading.len() - self.input_position as usize - 1);
        }
    }

    pub fn remove_next_char_from_input(&mut self) {
        if self.input_position as usize != 0 {
            self.input_downloading.remove(self.input_downloading.len() - self.input_position as usize);
            self.input_position = self.input_position - 1;
        }
    }

    pub fn left_input_position(&mut self) {
        if self.input_position < self.input_downloading.len() as u16 {
            self.input_position = self.input_position + 1;
        }
    }

    pub fn right_input_position(&mut self) {
        if self.input_position > 0 {
            self.input_position = self.input_position - 1;
        }
    }
}

impl Service for DownloadingService {
    fn new(service_name: ServiceName) -> Self {
        DownloadingService {
            service_name: service_name,
            input_downloading: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
            state_download: 0.0,
            input_position: 0
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
        
        let download_border_style = if active_service == self.get_name() {Color::Magenta} else {Color::Reset};
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

        // Render cursor only if service is the active one
        if active_service == self.get_name() {
            frame.set_cursor_position(Position::new(
                // Right position - Left position to get the most inputable value
                // Adding the length of the input
                // Divide the all by 2 because the input set to the center
                // Adding 4 because of the 2 border + the last character + after it
                ((chunks[0].right() - chunks[0].left()) + self.input_downloading.len() as u16) / 2 + 4 - self.input_position,
                chunks[0].top()
            ));
        }
    }
}
