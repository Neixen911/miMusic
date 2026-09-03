use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Row, Table, TableState},
    Frame
};

use crate::api::{self, Playlist};
use crate::tools::{Answer, PopupTool};
use crate::ui::{PopupState, Service, ServiceName};

pub struct PlaylistsService {
    service_name: ServiceName,
    active_playlist: String,
    playlists_state: TableState,
    add_popup: PopupTool,
    modify_popup: PopupTool,
    delete_popup: PopupTool
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

    pub fn get_selected_playlist(&mut self) -> String {
        self.get_all_playlists()[self.get_playlists_state()].playlist_name.clone()
    }

    pub fn get_all_playlists(&mut self) -> Vec<Playlist> {
        api::get_all_playlists()
    }

    pub fn get_add_popup(&mut self) -> PopupTool {
        self.add_popup.clone()
    }

    pub fn set_modify_popup(&mut self, playlist_to_modify: &String) {
        self.modify_popup.set_input(String::from(playlist_to_modify));
    }

    pub fn get_modify_popup(&mut self) -> &mut PopupTool {
        &mut self.modify_popup
    }

    pub fn set_delete_popup(&mut self, playlist_to_delete: &String) {
        let question = format!("Do you really want to delete '{}' playlist ?", playlist_to_delete);
        self.delete_popup = PopupTool::new(question, Answer::BINARY(String::from("Yes"), String::from("No")));
    }

    pub fn get_delete_popup(&mut self) -> PopupTool {
        self.delete_popup.clone()
    }
}

impl Service for PlaylistsService {
    fn new(service_name: ServiceName) -> Self {
        PlaylistsService {
            service_name: service_name,
            active_playlist: "All songs".to_string(),
            playlists_state: TableState::default().with_selected(0),
            add_popup: PopupTool::new(format!("Do you want to add a new playlist ?"), Answer::BINARY(String::from("Yes"), String::from("No"))),
            modify_popup: PopupTool::new(format!("What's the new name of the selected playlist ?"), Answer::INPUT(String::from("Playlist"))),
            delete_popup: PopupTool::new(format!("Do you really want to delete this playlist ?"), Answer::BINARY(String::from("Yes"), String::from("No")))
        }
    }

    fn get_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn handle_popup_events(&mut self, key_event: KeyEvent, mode: &PopupState) {
        match mode {
            &PopupState::ADD                => { self.add_popup.handle_popup_events(key_event); },
            &PopupState::MODIFY             => { self.modify_popup.handle_popup_events(key_event); },
            &PopupState::DELETE             => { self.delete_popup.handle_popup_events(key_event); },
            _ => {}
        }
    }

    fn handle_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Enter                  => { self.set_active_playlist(); },
            KeyCode::Up                     => { self.previous(); },
            KeyCode::Down                   => { self.next(); },
            _ => {}
        }
    }

    fn update(&mut self) {}

    fn render_popups(&mut self, frame: &mut Frame, mode: &PopupState) {
        match mode {
            &PopupState::ADD                => { self.add_popup.render(frame); },
            &PopupState::MODIFY             => { self.modify_popup.render(frame); },
            &PopupState::DELETE             => { self.delete_popup.render(frame); },
            _ => {}
        }
    }

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
