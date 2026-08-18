use super::{Service, ServiceName};
use crate::domain;
use crate::tools::InputTool;

use ratatui::{
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Gauge},
    Frame
};
use tokio;
use tokio::sync::watch::{self, Receiver, Sender};

pub struct DownloadService {
    pub service_name: ServiceName,
    pub sender: Sender<(u32, u32, u32, f64)>,
    pub receiver: Receiver<(u32, u32, u32, f64)>,
    pub input: InputTool,
    pub started: bool,
    pub persistent_percent: f64
}

impl DownloadService {
    pub fn download(&mut self) {
        self.started = true;
        domain::download_song(self.sender.clone(), self.get_input(), String::from("All songs"));
        domain::normalize_song(self.sender.clone());
    }

    // Update progress bar when downloading song(s)
    pub fn update_download_status(&mut self) {
        // Check a changer avec has_changed() ou changed().await
        if self.started == true {
            let (intitule, downloading_index, downloading_total, downloading_percent) = *self.receiver.borrow();
            // Intitule 1: Download
            if intitule == 1 {
                let mut index_of_total = String::from("");
                if downloading_index != 0 && downloading_total != 0 {
                    index_of_total = format!("{}/{}", downloading_index, downloading_total);
                }

                match downloading_percent {
                    x if (0.0..=100.0).contains(&x) => {
                        self.persistent_percent = downloading_percent / 100.0;
                        self.set_input(format!("Download {}: {}%", index_of_total, downloading_percent));
                    }
                    x if x == -1.0 => {
                        self.persistent_percent = 0.0;
                        self.set_input("Download successfull !".to_string());
                    }
                    x if x == -2.0 => {
                        self.set_input("Installing librairies ...".to_string());
                    }
                    x if x == -3.0 => {
                        self.set_input("Starting to fetch datas from YouTube URL ...".to_string());
                    }
                    x if x == -4.0 => {
                        self.set_input("Check integrity and finalize ...".to_string());
                    }
                    x if x == -51.0 => {
                        self.persistent_percent = 0.0;
                        self.set_input("Skip already downloaded songs and other ones successfully downloaded !".to_string());
                    }
                    x if x == -98.0 => {
                        self.started = false;
                        self.set_input("No internet connection !".to_string());
                    }
                    x if x == -99.0 => {
                        self.started = false;
                        self.set_input("Unsupported architecture ! Please report it to making an issue in Github !".to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn update_normalization_status(&mut self) {
        if self.started == true {
            let (intitule, normalizing_index, normalizing_total, normalizing_percent) = *self.receiver.borrow();
            // Intitule 2: Normalize
            if intitule == 2 {
                let mut index_of_total = String::from("");
                if normalizing_index != 0 && normalizing_total != 0 {
                    index_of_total = format!("{}/{}", normalizing_index, normalizing_total);
                }

                match normalizing_percent {
                    x if (0.0..=100.0).contains(&x) => {
                        self.persistent_percent = normalizing_percent / 100.0;
                        self.set_input(format!("Normalize {}: {}%", index_of_total, normalizing_percent));
                    }
                    x if x == -1.0 => {
                        self.persistent_percent = 0.0;
                        self.started = false;
                        self.set_input("Normalization successfull !".to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn left_input_position(&mut self) {
        self.input.left_input_position();
    }

    pub fn right_input_position(&mut self) {
        self.input.right_input_position();
    }

    pub fn remove_previous_char_from_input(&mut self) {
        self.input.remove_previous_char_from_input();
    }

    pub fn remove_next_char_from_input(&mut self) {
        self.input.remove_next_char_from_input();
    }

    pub fn add_char_to_input(&mut self, new_char: char) {
        self.input.add_char_to_input(new_char);
    }

    pub fn set_input(&mut self, new_input: String) {
        self.input.set_input(new_input);
    }

    pub fn get_input(&mut self) -> String {
        self.input.get_input()
    }

    pub fn get_input_position(&mut self) -> usize {
        self.input.get_position()
    }
}

impl Service for DownloadService {
    fn new(service_name: ServiceName) -> Self {
        // Active number of download, total number of download, percent of download
        let (sender, receiver) = watch::channel((0, 0, 0, 0.0));
        DownloadService {
            service_name: service_name,
            sender: sender,
            receiver: receiver,
            input: InputTool::new("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()),
            started: false,
            persistent_percent: 0.0
        }
    }

    fn get_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn update(&mut self) {
        self.update_download_status();
        self.update_normalization_status();
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

        let input = self.get_input();
        let downloading_label = Span::styled(
            &input,
            Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC),
        );
        let downloading_gauge_section = Gauge::default()
            .ratio(self.persistent_percent)
            .gauge_style(Color::Magenta)
            .label(downloading_label);
        frame.render_widget(downloading_gauge_section, chunks[0]);

        if is_active {
            frame.set_cursor_position(Position::new(
                ((chunks[0].right() - chunks[0].left()) + self.get_input().len() as u16) / 2 + 4 - self.get_input_position() as u16,
                chunks[0].top()
            ));
        }
    }
}
