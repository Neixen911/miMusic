use super::Service;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Gauge, Paragraph},
    Frame
};

pub struct PlayingService {
    pub playing_infos: Vec<String>
}

impl PlayingService {
    pub fn new() -> Self {
        PlayingService {
            playing_infos: Vec::new()
        }
    }

    // Convert seconds to minutes/seconds
    pub fn seconds_to_minsec(seconds: f64) -> (u32, u32) {
        let min = (seconds / 60.0).floor() as u32;
        let sec = (seconds % 60.0).round() as u32;

        (min, sec)
    }
}

impl Service for PlayingService {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(4),              // Playing informations
            Constraint::Length(1),              // Duration gauge
        ])
        .margin(1)
        .split(area);

        let playing_section = Block::default()
            .title(Line::from("Now Playing"))
            .borders(ratatui::widgets::Borders::ALL);
        frame.render_widget(playing_section, area);

        let mut playing_lines: Vec<Line> = Vec::new();
        playing_lines.push(Line::from(self.playing_infos.get(0).expect("Unable to get title from current playing song !").to_string()));
        playing_lines.push(Line::from(self.playing_infos.get(1).expect("Unable to get artist from current playing song !").to_string()));
        let infos_section = Paragraph::new(playing_lines);
        frame.render_widget(infos_section, chunks[0]);

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
