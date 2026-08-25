use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Row, Table, TableState},
    Frame
};

use crate::{Service, ServiceName};
use crate::api::{self, Playlist};

#[derive(Debug)]
pub struct PlaylistsService {
    service_name: ServiceName,
    active_playlist: String,
    playlists_state: TableState
}

impl PlaylistsService {
    pub fn toggle_playlists(&mut self, path: String, selected_playlist: &String) {
        api::toggle_playlists(path, selected_playlist);
    }

    pub fn add_playlist(&mut self) {
        api::add_playlist();
    }

    pub fn modify_playlist(&mut self, actual_playlist_position: usize, new_playlist_name: &String) {
        api::modify_playlist(actual_playlist_position, new_playlist_name);
    }

    pub fn remove_playlist(&mut self, playlist_position_to_remove: usize) {
        api::remove_playlist(playlist_position_to_remove);
    }

    // Select previous playlist in playlists table
    pub fn previous(&mut self) {
        let mut selected_playlist_id = self.get_playlists_state();
        if selected_playlist_id == 0 {
            selected_playlist_id = self.get_all_playlists().len() - 1;
        } else {
            selected_playlist_id -= 1;
        }
        self.set_playlists_state(selected_playlist_id);
    }

    // Select next playlist in playlists table
    pub fn next(&mut self) {
        let mut selected_playlist_id = self.get_playlists_state();
        if selected_playlist_id >= self.get_all_playlists().len() - 1 {
            selected_playlist_id = 0;
        } else {
            selected_playlist_id += 1;
        }
        self.set_playlists_state(selected_playlist_id);
    }

    pub fn set_active_playlist(&mut self) {
        let new_active_playlist = &self.get_all_playlists()[self.get_playlists_state()].playlist_name;
        self.active_playlist = new_active_playlist.to_string();
    }

    pub fn get_active_playlist(&mut self) -> &String {
        &self.active_playlist
    }

    pub fn set_playlists_state(&mut self, new_playlist_state: usize) {
        self.playlists_state.select(Some(new_playlist_state));
    }

    pub fn get_playlists_state(&mut self) -> usize {
        self.playlists_state.selected().expect("Can't retrieve active playlist selected id !")
    }

    pub fn get_all_playlists(&mut self) -> Vec<Playlist> {
        api::get_all_playlists()
    }
}

impl Service for PlaylistsService {
    fn new(service_name: ServiceName) -> Self {
        PlaylistsService {
            service_name: service_name,
            active_playlist: "All songs".to_string(),
            playlists_state: TableState::default().with_selected(0),
        }
    }

    fn as_any(&mut self) -> &mut dyn Any { self }

    fn get_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn update(&mut self) {}

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName) {
        let mut playlists_datas: Vec<Row> = Vec::new();
        let all_playlists = self.get_all_playlists();
        for playlist in &all_playlists {
            playlists_datas.push(Row::new(vec![
                playlist.playlist_name.as_str(),
            ]));
        }
        let playlists_border_style = if active_service == self.get_name() {Color::Magenta} else {Color::Reset};
        let playlists_table = Table::new(
            playlists_datas,
            [
                Constraint::Fill(1),
            ])
            .block(
                Block::default()
                .title(Line::from("Playlists"))
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(playlists_border_style)
            )
            .row_highlight_style(Style::default().fg(Color::Magenta))
            .highlight_symbol(Text::from(vec![" █ ".into()]));
        frame.render_stateful_widget(playlists_table, area, &mut self.playlists_state);
    }
}
