#![feature(str_split_remainder)]
#![feature(path_is_empty)]

mod settings;
mod api;
mod ui;
mod tools;

use dotenv::dotenv;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout},
    text::Line,
    widgets::Block,
    DefaultTerminal, Frame,
};
use std::fs::{self, File, read_to_string};
use std::io::{self, BufWriter, Write};
use std::time::{Duration, Instant};
use tokio;

use crate::ui::{DownloadService, PlayerService, PlaylistsService, PopupState, Service, ServiceName, SongsService};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App {
        active_service: ServiceName::SONGS,
        mode: PopupState::NONE,
        player_service: PlayerService::new(ServiceName::PLAYER),
        download_service: DownloadService::new(ServiceName::DOWNLOAD),
        playlists_service: PlaylistsService::new(ServiceName::PLAYLISTS),
        songs_service: SongsService::new(ServiceName::SONGS),
        is_running: false,
    };
    let running_app = app.run(&mut terminal).await;
    ratatui::restore();
    running_app
}

pub struct App {
    active_service: ServiceName,
    mode: PopupState,
    player_service: PlayerService,
    download_service: DownloadService,
    playlists_service: PlaylistsService,
    songs_service: SongsService,
    is_running: bool,
}

impl App {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.is_running = true;
        dotenv().ok();
        self.load_settings();

        let tick_rate = Duration::from_millis(250);
        let mut tick_time_elapsed = Instant::now();

        while self.is_running {
            // Draw TUI
            terminal.draw(|frame| self.draw(frame))?;

            // Do not wait keys events more than 0.25s after render TUI
            let timeout = tick_rate.saturating_sub(tick_time_elapsed.elapsed());
            if event::poll(timeout).expect("Can't check if event::poll during timeout value !") {
                self.handle_events().await;
            }

            // Update datas each 0.25s (not each frame bc it makes 10x CPU usage)
            if tick_time_elapsed.elapsed() >= tick_rate {
                self.update_datas().await;
                tick_time_elapsed = Instant::now();
            }
        }
        Ok(())
    }

    // Function to update all datas
    async fn update_datas(&mut self) {
        self.player_service.update();
        self.download_service.update();
        self.songs_service.active_playlist = self.playlists_service.get_active_playlist().to_string();
    }

    // Retrieve keys events
    async fn handle_events(&mut self) {
        match event::read().expect("Can't read events !") {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                if self.mode == PopupState::NONE {
                    self.handle_main_events(key_event).await;
                } else { self.handle_popup_events(key_event).await };
            }
            _ => {}
        };
    }

    // Match popups key event to dedicated service
    async fn handle_popup_events(&mut self, key_event: KeyEvent) {
        let mode = &self.mode;
        match self.active_service {
            ServiceName::SONGS => {
                match self.mode {
                    PopupState::ADD => {
                        match key_event.code {
                            KeyCode::Enter                  => { self.toggle_playlists(); },
                            _                               => { self.songs_service.handle_popup_events(key_event, mode); }
                        }
                    },
                    PopupState::MODIFY => {
                        match key_event.code {
                            KeyCode::Left                   => { self.songs_service.get_modify_popup().get_input_tool().left_input_position(); },
                            KeyCode::Right                  => { self.songs_service.get_modify_popup().get_input_tool().right_input_position(); },
                            KeyCode::Backspace              => { self.songs_service.get_modify_popup().get_input_tool().remove_previous_char_from_input(); },
                            KeyCode::Delete                 => { self.songs_service.get_modify_popup().get_input_tool().remove_next_char_from_input(); },
                            KeyCode::Char(to_insert)        => { self.songs_service.get_modify_popup().get_input_tool().add_char_to_input(to_insert); },
                            KeyCode::Enter => {
                                let selected_song = self.songs_service.get_selected_song();
                                let actual_modified_intitule = self.songs_service.get_modify_popup().get_state().expect("Can't retrieve actual id of selected song !");
                                let filepath = selected_song.clone()
                                    .expect("Can't be a None value !")
                                    .get("path")
                                    .expect("Can't retrieve path of song file !")
                                    .to_string();
                                let song_last_intitule = self.songs_service.get_modify_metadata(filepath).len() - 1;
                                if actual_modified_intitule < song_last_intitule {
                                    self.songs_service.get_modify_popup().next();
                                } else {
                                    self.songs_service.get_modify_popup().save();
                                    let new_metadata = self.songs_service.get_modify_popup().get_answers();
                                    self.songs_service.set_metadata(
                                        selected_song
                                            .expect("Can't be a None value !")
                                            .get("path")
                                            .expect("Can't retrieve path of song file !")
                                            .to_string(),
                                        &new_metadata
                                    );
                                    self.set_mode(PopupState::NONE);
                                }
                            },
                            _ => { self.songs_service.handle_popup_events(key_event, mode); }
                        }
                    },
                    PopupState::DELETE => {
                        match key_event.code {
                            KeyCode::Enter => {
                                if self.songs_service.get_delete_popup().is_positive() {
                                    let song_to_delete = self.songs_service.get_selected_song()
                                        .expect("Can't be a None value!")
                                        .get("path")
                                        .expect("Can't retrieve path of the selected song !")
                                        .to_string();
                                    self.songs_service.remove_song(song_to_delete);
                                }
                                self.set_mode(PopupState::NONE);
                            },
                            _ => { self.songs_service.handle_popup_events(key_event, mode); }
                        }
                    },
                    _ => {}
                }
            },
            ServiceName::PLAYER => {
                match key_event.code {
                    _                                       => { self.player_service.handle_popup_events(key_event, mode); }
                }
            },
            ServiceName::DOWNLOAD => {
                match key_event.code {
                    _                                       => { self.download_service.handle_popup_events(key_event, mode); }
                }
            },
            ServiceName::PLAYLISTS => {
                match self.mode {
                    PopupState::ADD => {
                        match key_event.code {
                            KeyCode::Enter => {
                                if self.playlists_service.get_add_popup().is_positive() {
                                    self.playlists_service.add_playlist();
                                }
                                self.set_mode(PopupState::NONE);
                            },
                            _ => { self.playlists_service.handle_popup_events(key_event, mode); }
                        }
                    },
                    PopupState::MODIFY => {
                        match key_event.code {
                            KeyCode::Left                   => { self.playlists_service.get_modify_popup().get_input_tool().left_input_position(); },
                            KeyCode::Right                  => { self.playlists_service.get_modify_popup().get_input_tool().right_input_position(); },
                            KeyCode::Backspace              => { self.playlists_service.get_modify_popup().get_input_tool().remove_previous_char_from_input(); },
                            KeyCode::Delete                 => { self.playlists_service.get_modify_popup().get_input_tool().remove_next_char_from_input(); },
                            KeyCode::Char(to_insert)        => { self.playlists_service.get_modify_popup().get_input_tool().add_char_to_input(to_insert); },
                            KeyCode::Enter => {
                                let i = self.playlists_service.get_playlists_state();
                                let playlist_to_modify = self.playlists_service.get_modify_popup().get_input_tool().get_input();
                                self.playlists_service.modify_playlist(i, &playlist_to_modify);
                                self.set_mode(PopupState::NONE);
                            }
                            _ => { self.playlists_service.handle_popup_events(key_event, mode); }
                        }
                    },
                    PopupState::DELETE => {
                        match key_event.code {
                            KeyCode::Enter => {
                                if self.playlists_service.get_delete_popup().is_positive() {
                                    let playlist_to_delete = self.playlists_service.get_playlists_state();
                                    self.playlists_service.remove_playlist(playlist_to_delete);
                                }
                                self.set_mode(PopupState::NONE);
                            },
                            _ => { self.playlists_service.handle_popup_events(key_event, mode); }
                        }
                    },
                    _ => {}
                }
            }
        }
        
        match key_event.code {
            KeyCode::Esc                            => { self.set_mode(PopupState::NONE); },
            _ => {}
        }
    }

    // Match main key event to dedicated service
    async fn handle_main_events(&mut self, key_event: KeyEvent) {
        match self.active_service {
            ServiceName::SONGS => {
                match key_event.code {
                    KeyCode::Char('q')              => { self.exit(); },
                    KeyCode::Tab                    => {
                        self.songs_service.set_songs_state(None);
                        self.active_service = ServiceName::PLAYER;
                    },
                    KeyCode::Enter                  => { self.add_song_to_queue(); },
                    KeyCode::Char('m') => {
                        let selected_song = self.songs_service.get_selected_song();
                        if selected_song.is_some() {
                            let filepath = selected_song
                                .expect("Can't be a None value !")
                                .get("path")
                                .expect("Can't retrieve path of song file !")
                                .to_string();
                            let song_infos = self.songs_service.get_modify_metadata(filepath);
                            self.songs_service.set_modify_popup(song_infos);
                            self.set_mode(PopupState::MODIFY);
                        }
                    },
                    KeyCode::Char('l')              => { self.set_favorites(); },
                    KeyCode::Char('a') => {
                        let song = self.songs_service.get_selected_song();
                        if song.is_some() {
                            self.songs_service.set_add_popup(song.clone().expect("Can't be a None value !"));
                            self.set_mode(PopupState::ADD);
                        }
                    },
                    KeyCode::Delete => {
                        let selected_song = self.songs_service.get_selected_song();
                        if selected_song.is_some() {
                            let song_name = selected_song
                                .expect("Can't be a None value !")
                                .get("title")
                                .expect("Can't retrieve title of the song !")
                                .to_string();
                            self.songs_service.set_delete_popup(&song_name);
                            self.set_mode(PopupState::DELETE);
                        }
                    },
                    _                               => { self.songs_service.handle_events(key_event); }
                }
            }
            ServiceName::PLAYER => {
                match key_event.code {
                    KeyCode::Char('q')              => { self.exit(); },
                    KeyCode::Tab                    => { self.active_service = ServiceName::DOWNLOAD; },
                    _                               => { self.player_service.handle_events(key_event); }
                }
            }
            ServiceName::DOWNLOAD => {
                match key_event.code {
                    KeyCode::Tab                    => { self.active_service = ServiceName::PLAYLISTS; },
                    _                               => { self.download_service.handle_events(key_event); }
                }
            }
            ServiceName::PLAYLISTS => {
                match key_event.code {
                    KeyCode::Char('q')              => { self.exit(); },
                    KeyCode::Tab                    => {
                        self.songs_service.set_songs_state(Some(0));
                        self.active_service = ServiceName::SONGS;
                    },
                    KeyCode::BackTab                => { self.add_all_songs_to_queue(); },
                    KeyCode::Char('a')              => { self.set_mode(PopupState::ADD); },
                    KeyCode::Char('m') => {
                        let selected_playlist = &self.playlists_service.get_selected_playlist();
                        if *selected_playlist != "All songs".to_string() && *selected_playlist != "Favorites".to_string() {
                            self.playlists_service.set_modify_popup(selected_playlist);
                            self.set_mode(PopupState::MODIFY);
                        }
                    },
                    KeyCode::Delete => {
                        let selected_playlist = &self.playlists_service.get_selected_playlist();
                        if *selected_playlist != "All songs".to_string() && *selected_playlist != "Favorites".to_string() {
                            self.playlists_service.set_delete_popup(selected_playlist);
                            self.set_mode(PopupState::DELETE);
                        }
                    },
                    _                               => { self.playlists_service.handle_events(key_event); }
                }
            }
        }
    }

    // Load all the settings from the settings file
    pub fn load_settings(&mut self) {
        if !fs::exists("settings.json").expect("Non authorized folder check !") {
            let mut initialisation: Vec<(String, f32)> = Vec::new();
            initialisation.push(
                ("Volume".to_string(), 1.0)
            );

            let settings_file = File::create("settings.json").expect("Failed to create/open settings.json");
            let mut settings_writer = BufWriter::new(settings_file);
            let _ = serde_json::to_writer_pretty(&mut settings_writer, &initialisation);
            let _ = settings_writer.flush();
        }
        let settings_content = read_to_string("settings.json").expect("Can't read content of settings.json file !");
        let settings: Vec<(String, f32)> = serde_json::from_str(&settings_content)
            .expect("Settings JSON content is not well-formatted !");
        for (key, value) in settings {
            match key.as_str() {
                "Volume" => {
                    self.player_service.set_volume_manual(value);
                }
                &_ => {
                    println!("{}: {}", key, value);
                    continue;
                }
            }
        }
    }

    // SONGS service needed PLAYER service to add song to the queue
    fn add_song_to_queue(&mut self) {
        let selected_song = self.songs_service.get_selected_song();
        if selected_song.is_some() {
            let path = selected_song.expect("Cannot be a None value !")
                .get("path")
                .expect("Can't retrieve path of file song !")
                .to_string();
            self.player_service.add_song_to_queue(path);
        }
    }

    // PLAYLISTS service needed SONGS service to retrieve the selected song and PLAYER service to add it to the queue
    fn add_all_songs_to_queue(&mut self) {
        let songs_to_add = self.songs_service.get_all_songs();
        for song in songs_to_add {
            self.player_service.add_song_to_queue(song.get("path").expect("Can't retrieve path of file song !").to_string());
        }
    }

    // PLAYLISTS service needed SONGS service to retrieve active song to add to playlist
    fn toggle_playlists(&mut self) {
        let selected_song = self.songs_service.get_selected_song();
        if selected_song.is_some() {
            let song_path = selected_song
                .expect("Can't be a None value !")
                .get("path")
                .expect("Can't retrieve path of the selected song !")
                .to_string();
            let selected_playlist = &self.songs_service.get_add_popup().get_answer();
            if selected_playlist != "All songs" {
                self.playlists_service.toggle_playlists(song_path, selected_playlist);
            }
        }
    }

    // PLAYLISTS service needs SONGS service to retrieve active song and add it to 'Favorites' playlist
    fn set_favorites(&mut self) {
        let selected_song = self.songs_service.get_selected_song();
        if selected_song.is_some() {
            let path = selected_song
                .expect("Can't be a None value !")
                .get("path")
                .expect("Can't retrieve path of the selected song !")
                .to_string();
            self.playlists_service.toggle_playlists(path, &"Favorites".to_string());
        }
    }

    fn set_mode(&mut self, new_mode: PopupState) {
        self.mode = new_mode;
    }

    fn render_popups(&mut self, frame: &mut Frame) {
        let mode = &self.mode;
        match self.active_service {
            ServiceName::PLAYER         => { self.player_service.render_popups(frame, mode); },
            ServiceName::DOWNLOAD       => { self.download_service.render_popups(frame, mode); },
            ServiceName::PLAYLISTS      => { self.playlists_service.render_popups(frame, mode); },
            ServiceName::SONGS          => { self.songs_service.render_popups(frame, mode); },
        }
    }

    // Draw TUI app
    fn draw(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),              // Application section
            Constraint::Length(5),              // Playing section
            Constraint::Length(3),              // Download section
            Constraint::Fill(1),                // Playlists/Songs section
            Constraint::Length(1),              // Hotkeys section
        ]).margin(3);
        let [app, playing, download, playlists_songs, hotkeys] = vertical.areas(frame.area());

        // Application section
        let app_text = Block::default()
            .title(Line::from(" miMusic ").centered());
        frame.render_widget(app_text, app);
        
        // Playing section
        self.player_service.render(frame, playing, &self.active_service);

        // Downloading section
        self.download_service.render(frame, download, &self.active_service);

        // Playlists & Songs section
        let horizontal = Layout::horizontal([
            Constraint::Fill(1),              // Playlists section
            Constraint::Fill(4),              // Songs section
        ]);
        let [playlists, songs] = horizontal.areas(playlists_songs);

        self.playlists_service.render(frame, playlists, &self.active_service);
        self.songs_service.render(frame, songs, &self.active_service);

        self.render_popups(frame);

        // Hotkeys section
        let hotkeys_text: String;
        let mode = &self.mode;
        match self.active_service {
            ServiceName::PLAYER         => { hotkeys_text = self.player_service.get_hotkeys(mode); },
            ServiceName::DOWNLOAD       => { hotkeys_text = self.download_service.get_hotkeys(mode); },
            ServiceName::PLAYLISTS      => { hotkeys_text = self.playlists_service.get_hotkeys(mode); },
            ServiceName::SONGS          => { hotkeys_text = self.songs_service.get_hotkeys(mode); },
        }
        let hotkeys_section = Block::default()
            .title(Line::from(hotkeys_text).centered());
        frame.render_widget(hotkeys_section, hotkeys);
    }

    // Exit the app on key pressed
    fn exit(&mut self) {
        self.is_running = false;
    }
}
