use super::Service;
use super::super::music::Playlist;

use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Row, Table, TableState},
    Frame
};

pub struct PlaylistsService {
    active_playlist: String,
    all_playlists: Vec<Playlist>,
    playlists_state: TableState
}

impl PlaylistsService {
    pub fn new() -> Self {
        PlaylistsService {
            active_playlist: "All songs".to_string(),
            all_playlists: Vec::new(),
            playlists_state: TableState::default().with_selected(0),
        }
    }

    pub fn set_active_playlist(&mut self, new_playlist: String) {
        self.active_playlist = new_playlist;
    }

    pub fn get_active_playlist(&self) -> &String {
        &self.active_playlist
    }

    pub fn set_all_playlists(&mut self, new_playlist_list: Vec<Playlist>) {
        self.all_playlists = new_playlist_list;
    }

    pub fn get_all_playlists(&self) -> &Vec<Playlist> {
        &self.all_playlists
    }

    pub fn set_playlists_state(&mut self, new_playlist_state: usize) {
        self.playlists_state.select(Some(new_playlist_state));
    }

    pub fn get_playlists_state(&self) -> usize {
        self.playlists_state.selected().expect("Can't retrieve active playlist selected id !")
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
}

impl Service for PlaylistsService {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let mut playlists_datas: Vec<Row> = Vec::new();
        for playlist in &self.all_playlists {
            playlists_datas.push(Row::new(vec![
                playlist.playlist_name.as_str(),
            ]));
        }
        //let playlists_border_style = if self.mode.as_str() == "playlists" {Color::Magenta} else {Color::Reset};
        let playlists_border_style = Color::Reset;
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
