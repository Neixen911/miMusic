use ratatui::{
    layout::{Constraint, Rect},
    prelude::{Alignment},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Cell, Row, Table, TableState},
    Frame
};
use std::any::Any;
use std::collections::HashMap;

use crate::{Service, ServiceName};
use crate::api;
use crate::settings;

pub struct SongsService {
    service_name: ServiceName,
    songs_state: TableState,
    // TODO: Retirer active_playlist d'ici et de Playlists service, puis gérer ça dans playlists.json avec clé/valeur
    pub active_playlist: String,
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

    pub fn remove_song(&mut self, song_to_remove: String) {
        api::remove_song(song_to_remove);
    }

    pub fn get_all_songs(&mut self) -> Vec<HashMap<String, String>> {
        api::get_all_songs(&self.active_playlist)
    }

    pub fn set_songs_state(&mut self, new_song_state: Option<usize>) {
        self.songs_state.select(new_song_state);
    }

    pub fn get_songs_state(&mut self) -> Option<usize> {
        self.songs_state.selected()
    }

    pub fn get_selected_song(&mut self) -> Option<HashMap<String, String>> {
        let i = self.get_songs_state();
        let mut song = None;
        if i.is_some() {
            song = Some(self.get_all_songs()[i.expect("Cannot be a None value !")].clone())
        }

        song
    }

    pub fn set_metadata(&mut self, filepath: String, new_song_datas: &Vec<(String, String)>) {
        api::set_metadata(filepath, new_song_datas);
    }

    pub fn get_modify_metadata(&mut self, filepath: String) -> Vec<(String, String)> {
		api::get_modify_metadata(filepath)
	}
}

impl Service for SongsService {
    fn new(service_name: ServiceName) -> Self {
        SongsService {
            service_name: service_name,
            songs_state: TableState::default().with_selected(0),
            active_playlist: "All songs".to_string(),
        }
    }

    fn as_any(&mut self) -> &mut dyn Any { self }

    fn get_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn update(&mut self) {}

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName) {
        let mut songs_datas: Vec<Row> = Vec::new();
        for song in self.get_all_songs() {
            let (min, sec) = settings::seconds_to_minsec(song.get("duration")
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

        // Popups gestion
        match active_service {
            ServiceName::SONGS(SongsInterface::ADD) => {
                if let Some(playlist_service_result) = registry.get_service::<PlaylistsService>() {
                    let mut playlist_service_global = playlist_service_result.lock().unwrap();
                    let playlist_service = playlist_service_global.as_any().downcast_mut::<PlaylistsService>().unwrap();
                    let selected_song_result = self.get_selected_song();
                    if selected_song_result.is_some() {
                        let selected_song = selected_song_result.expect("Can't retrieve selected song !");
                        let popup_question = format!(
                            "In which playlist(s) do you want to add '{}' song ?",
                            selected_song.get("title").expect("Can't have an empty title name song !")
                        );
                        let mut playlists_datas: Vec<(String, String)> = Vec::new();
                        for playlist in playlist_service.get_all_playlists() {
                            let is_in_playlist = playlist.songs_list.contains(selected_song.get("path").expect("Can't retrieve path of song file !"));
                            let checkbox: &str;
                            if is_in_playlist || playlist.playlist_name == "All songs".to_string() {
                                checkbox = "[X]";
                            } else { checkbox = "[ ]"; }
                            playlists_datas.push((
                                checkbox.to_string(),
                                playlist.playlist_name.clone()
                            ));
                        }
                        let mut popup = PopupTool::new(
                            popup_question,
                            Answer::TABLE(playlists_datas)
                        );
                        popup.render(frame);
                    }
                }
            },
            ServiceName::SONGS(SongsInterface::MODIFY) => {

            },
            ServiceName::SONGS(SongsInterface::DELETE) => {

            },
            _ => {}
        }
    }
}
