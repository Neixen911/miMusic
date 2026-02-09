use super::{Service, ServiceName, SongsInterface};

use ratatui::{
    layout::{Constraint, Rect},
    prelude::{Alignment},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Cell, Row, Table, TableState},
    Frame
};
use std::collections::HashMap;

pub struct SongsService {
    all_songs: Vec<HashMap<String, String>>,
    songs_state: TableState
}

impl SongsService {
    pub fn new() -> Self {
        SongsService {
            all_songs: Vec::new(),
            songs_state: TableState::default().with_selected(0),
        }
    }

    // Select previous song in songs table
    pub fn previous(&mut self) {
        let mut selected_song_id = self.get_songs_state().expect("Can't retrieve active song id !");
        if selected_song_id == 0 {
            selected_song_id = self.get_all_songs().len() - 1;
        } else {
            selected_song_id -= 1;
        }
        self.set_songs_state(Some(selected_song_id));
    }

    // Select next song in songs table
    pub fn next(&mut self) {
        let mut selected_song_id = self.get_songs_state().expect("Can't retrieve active song id !");
        if selected_song_id >= self.get_all_songs().len() - 1 {
            selected_song_id = 0;
        } else {
            selected_song_id += 1;
        }
        self.set_songs_state(Some(selected_song_id));
    }

    // Convert seconds to minutes/seconds
    fn seconds_to_minsec(seconds: f64) -> (u32, u32) {
        let min = (seconds / 60.0).floor() as u32;
        let sec = (seconds % 60.0).round() as u32;

        (min, sec)
    }

    pub fn set_all_songs(&mut self, all_songs: Vec<HashMap<String, String>>) {
        self.all_songs = all_songs;
    }

    pub fn get_all_songs(&self) -> &Vec<HashMap<String, String>> {
        &self.all_songs
    }

    pub fn set_songs_state(&mut self, new_song_state: Option<usize>) {
        self.songs_state.select(new_song_state);
    }

    pub fn get_songs_state(&self) -> Option<usize> {
        self.songs_state.selected()
    }
}

impl Service for SongsService {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let mut songs_datas: Vec<Row> = Vec::new();
        for song in &self.all_songs {
            let (min, sec) = Self::seconds_to_minsec(song.get("duration")
                .expect("Unable to get song duration !")
                .to_string()
                .parse::<f64>()
                .expect("Unable to convert into f64 !"));
            let duration = format!("{:02}", min) + ":" + format!("{:02}", sec).as_str();
            songs_datas.push(Row::new(vec![
                Cell::from(Text::from(song.get("title").expect("Unable to get title from song !").to_string())),
                Cell::from(Text::from(song.get("artist").expect("Unable to get artist from song !").to_string())),
                Cell::from(Text::from(duration)),
                Cell::from(Text::from(song.get("is_favorite").expect("Unable to get is_favorite from song !").to_string()).alignment(Alignment::Center)),
            ]));
        }
        let header = Row::new(vec!["Title", "Artist", "Duration", ""]);
        // let songs_border_style = if self.mode.as_str() == "songs" {Color::Magenta} else {Color::Reset};
        let songs_border_style = Color::Reset;
        let songs_table = Table::new(
            songs_datas,
            [
                Constraint::Fill(2),                // Song name
                Constraint::Fill(1),                // Song's artists
                Constraint::Max(10),                // Song duration
                Constraint::Max(10),                // Is in favorites or not
            ])
            .block(
                Block::default()
                .title(Line::from("Songs"))
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(songs_border_style)
            )
            .header(header)
            .row_highlight_style(Style::default().fg(Color::Magenta))
            .highlight_symbol(Text::from(vec![" █ ".into()]));
        frame.render_stateful_widget(songs_table, area, &mut self.songs_state);

        // Popups gestion
        match active_service {
            ServiceName::SONGS(SongsInterface::ADD) => {
                
            },
            ServiceName::SONGS(SongsInterface::MODIFY) => {

            },
            ServiceName::SONGS(SongsInterface::DELETE) => {

            },
            _ => {}
        }
    }
}
