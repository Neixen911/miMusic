use super::{Service, ServiceName};

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
    service_name: ServiceName,
    all_songs: Vec<HashMap<String, String>>,
    songs_state: TableState
}

impl SongsService {
    // Select previous song in songs table
    pub fn previous(&mut self) {
        let selected_song_id_state = self.get_songs_state();
        if selected_song_id_state.is_some() {
            let mut selected_song_id = selected_song_id_state.expect("Can't retrieve active song id !");
            if selected_song_id == 0 {
                selected_song_id = self.get_all_songs().len() - 1;
            } else {
                selected_song_id -= 1;
            }
            self.set_songs_state(Some(selected_song_id));
        }
    }

    // Select next song in songs table
    pub fn next(&mut self) {
        let selected_song_id_state = self.get_songs_state();
        if selected_song_id_state.is_some() {
            let mut selected_song_id = selected_song_id_state.expect("Can't retrieve active song id !");
            if selected_song_id >= self.get_all_songs().len() - 1 {
                selected_song_id = 0;
            } else {
                selected_song_id += 1;
            }
            self.set_songs_state(Some(selected_song_id));
        }
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

	pub fn get_modify_song_infos(&mut self) -> Vec<(String, String)> {
		let mut song_infos = Vec::new();
        let song = &self.get_all_songs()[self.get_songs_state().expect("Can't retrieve active song id !")];

        let song_entitled = ["TIT2", "TPE1"];
        let mut song_value: &str;

		for entitled_name in song_entitled {
            match entitled_name {
                "TIT2" => {
                    song_value = "title";
                }
                "TPE1" => {
                    song_value = "artist";
                }
                _default => {
                    continue;
                }
            }
            song_infos.push((entitled_name.to_string(), song.get(song_value).expect("Can't retrieve a specific value of a song !").to_string()));
		}

		song_infos
	}
}

impl Service for SongsService {
    fn new(service_name: ServiceName) -> Self {
        SongsService {
            service_name: service_name,
            all_songs: Vec::new(),
            songs_state: TableState::default().with_selected(0),
        }
    }

    fn get_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName) {
        let mut songs_datas: Vec<Row> = Vec::new();
        for song in self.get_all_songs() {
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
        let songs_border_style = if active_service == self.get_name() {Color::Magenta} else {Color::Reset};
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
    }
}
