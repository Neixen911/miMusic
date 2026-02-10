use super::{Service, ServiceName};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Gauge, Paragraph},
    Frame
};

pub struct PlayingService {
    pub service_name: ServiceName,
    pub playing_infos: Vec<String>,
    pub previously_playing_infos: Vec<String>
}

impl PlayingService {
    // Convert seconds to minutes/seconds
    fn seconds_to_minsec(seconds: f64) -> (u32, u32) {
        let min = (seconds / 60.0).floor() as u32;
        let sec = (seconds % 60.0).round() as u32;

        (min, sec)
    }
}

impl Service for PlayingService {
    fn new(service_name: ServiceName) -> Self {
        PlayingService {
            service_name: service_name,
            playing_infos: Vec::new(),
            previously_playing_infos: Vec::new(),
        }
    }

    fn get_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName) {
        let chunks = Layout::vertical([
            Constraint::Length(4),              // Playing informations
            Constraint::Length(1),              // Duration gauge
        ])
        .margin(1)
        .split(area);

        let playing_border_style = if active_service == self.get_name() {Color::Magenta} else {Color::Reset};
        let playing_section = Block::default()
            .title(Line::from("Now Playing"))
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(playing_border_style);
        frame.render_widget(playing_section, area);

        let rows_content: Vec<(String, String)> = vec![
            (
                self.playing_infos.get(0).expect("Unable to get title from current playing song !").to_string(),
                self.playing_infos.get(4).expect("Unable to get volume value from player !").to_string()
            ),
            (
                self.playing_infos.get(1).expect("Unable to get artist from current playing song !").to_string(),
                self.playing_infos.get(5).expect("Unable to get loop value from player !").to_string()
            )
        ];

        let playing_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); rows_content.len()])
            .split(chunks[0]);

        for (i, (left, right)) in rows_content.iter().enumerate() {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(50),     // Song information
                    Constraint::Min(0),         // Flexible space
                    Constraint::Length(15),     // Player information
                ])
                .split(playing_chunks[i]);

            frame.render_widget(Paragraph::new(Line::from(left.to_string())), cols[0]);
            frame.render_widget(Paragraph::new(Line::from(right.to_string())).right_aligned(), cols[2]);
        }

        let act_duration_song = self.playing_infos.get(2)
            .expect("Unable to get current duration from current playing song !")
            .to_string()
            .parse::<f64>()
            .expect("Unable to convert into f64 !");
        let max_duration_song = self.playing_infos.get(3)
            .expect("Unable to get maximum duration from current playing song !")
            .to_string()
            .parse::<f64>()
            .expect("Unable to convert into f64 !");
        let mut ratio = 0.0;
        let (act_minutes, act_seconds) = Self::seconds_to_minsec(act_duration_song);
        let (max_minutes, max_seconds) = Self::seconds_to_minsec(max_duration_song);
        let playing_label = Span::styled(
            format!("{:02}", act_minutes) 
            + ":" 
            + format!("{:02}", act_seconds).as_str() 
            + " / " 
            + format!("{:02}", max_minutes).as_str() 
            + ":" 
            + format!("{:02}", max_seconds).as_str(),
            Style::default(),
        );
        if max_duration_song != 0.0 {
            ratio = act_duration_song / max_duration_song;
        }
        let playing_gauge_section = Gauge::default()
            .ratio(ratio)
            .gauge_style(Color::Magenta)
            .label(playing_label);
        frame.render_widget(playing_gauge_section, chunks[1]);
    }
}
