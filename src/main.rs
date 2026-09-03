#![feature(str_split_remainder)]
#![feature(path_is_empty)]

mod settings;
mod api;
mod ui;
mod tools;

use dotenv::dotenv;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Alignment as TableAlignment, Constraint, Flex, Layout},
    prelude::{Alignment},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    DefaultTerminal, Frame,
};
use std::fs::{self, File, read_to_string};
use std::io::{self, BufWriter, Write};
use std::time::{Duration, Instant};
use tokio;

use crate::tools::InputTool;
use crate::ui::{DownloadService, PlayerService, PlaylistsService, PopupState, Service, ServiceName, SongsService};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App {
        song_to_playlists_state: TableState::default().with_selected(0),
        song_infos_state: TableState::default().with_selected(0),
        active_service: ServiceName::SONGS,
        player_service: PlayerService::new(ServiceName::PLAYER),
        download_service: DownloadService::new(ServiceName::DOWNLOAD),
        playlists_service: PlaylistsService::new(ServiceName::PLAYLISTS),
        songs_service: SongsService::new(ServiceName::SONGS),
        mode: "songs".to_string(),
        mode_popup: PopupState::NONE,
        input_song_datas: Vec::new(),
        input_modify_playlists: InputTool::new("".to_string()),
        is_running: false,
    };
    let running_app = app.run(&mut terminal).await;
    ratatui::restore();
    running_app
}

pub struct App {
    // Reorganise & reduce this variables (is_running at the end, some variables can maybe get out ...)
    song_to_playlists_state: TableState,
    song_infos_state: TableState,
    active_service: ServiceName,
    player_service: PlayerService,
    download_service: DownloadService,
    playlists_service: PlaylistsService,
    songs_service: SongsService,
    mode: String,
    mode_popup: PopupState,
    input_song_datas: Vec<(String, String)>,
    input_modify_playlists: InputTool,
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
                if self.mode_popup == PopupState::NONE {
                    self.handle_main_events(key_event).await;
                } else { self.handle_popup_events(key_event).await };
            }
            _ => {}
        };
    }

    // Match popups key event to dedicated service
    async fn handle_popup_events(&mut self, key_event: KeyEvent) {
        let mode = &self.mode_popup;
        match self.active_service {
            ServiceName::SONGS => {
                match self.mode_popup {
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
                match self.mode_popup {
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
            _ => {}
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
                // "modify_popup_songs" => {
                //     match key_event.code {
                //         KeyCode::Enter                  => { self.modify_song(); },
                //         KeyCode::Backspace              => { self.remove_char_from_input(); },
                //         KeyCode::Char(to_insert)        => { self.insert_char_into_input(to_insert); },
                //         KeyCode::Esc                    => { self.next_mode(); }
                //         _ => {}
                //     }
                // }
            _ => {}
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

    // Select next one in active table on key pressed
    fn next(&mut self) {
        match self.mode.as_str() {
            "modify_popup_songs" => {
                let i = match self.song_infos_state.selected() {
                    Some(i) => {
                        let filepath = self.songs_service.get_selected_song()
                            .expect("Can't be a None value !")
                            .get("path")
                            .expect("Can't retrieve path of song file !")
                            .to_string();
                        if i >= self.songs_service.get_modify_metadata(filepath).len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.song_infos_state.select(Some(i));
            }
            &_ => {}
        }
    }

    fn display_popup(&mut self, keyword: &str) {
        let selected_playlist = &self.playlists_service.get_all_playlists()[self.playlists_service.get_playlists_state()].playlist_name;
        match keyword {
            "modify_song" => {
                let selected_song = self.songs_service.get_selected_song();
                if selected_song.is_some() {
                    let filepath = selected_song
                        .expect("Can't be a None value !")
                        .get("path")
                        .expect("Can't retrieve path of song file !")
                        .to_string();
                    let modify_song_infos = self.songs_service.get_modify_metadata(filepath);
                    self.input_song_datas = Vec::new();
                    for (entitled_name, entitled_content) in modify_song_infos {
                        let entitled_name_formatted: String;
                        match entitled_name.as_str() {
                            "TIT2" => {
                                entitled_name_formatted = "Title".to_string();
                            }
                            "TPE1" => {
                                entitled_name_formatted = "Artist".to_string();
                            }
                            _default => {
                                continue;
                            }
                        }
                        self.input_song_datas.push((entitled_name_formatted, entitled_content));
                    }
                    let order_to_display = vec![
                        "Title".to_string(),
                        "Artist".to_string(),
                    ];
                    self.input_song_datas.sort_by_key(|(key, _)| {
                        order_to_display
                            .iter()
                            .position(|x| x == key)
                            .unwrap_or(usize::MAX)
                    });
                    self.song_infos_state.select(Some(0));
                    self.mode = "modify_popup_songs".to_string();
                }
            }
            &_ => {}
        }
    }

    // TODO: Revoir ça parce que c'est flou ce que ça fait ... (lu vite fait / en diagonale)
    fn modify_song(&mut self) {
        let selected_song = self.songs_service.get_selected_song();
        if selected_song.is_some() {
            let actual_modified_intitule = self.song_infos_state.selected().expect("Can't retrieve actual id of selected song !");
            let filepath = selected_song.clone()
                .expect("Can't be a None value !")
                .get("path")
                .expect("Can't retrieve path of song file !")
                .to_string();
            let song_last_intitule = self.songs_service.get_modify_metadata(filepath).len() - 1;
            if actual_modified_intitule < song_last_intitule {
                self.next();
            } else {
                let mut formatted_infos: Vec<(String, String)> = Vec::new();
                for (title, content) in &self.input_song_datas {
                    match title.as_str() {
                        "Title" => {
                            formatted_infos.push(("TIT2".to_string(), content.to_string()));
                        }
                        "Artist" => {
                            formatted_infos.push(("TPE1".to_string(), content.to_string()));
                        }
                        _default => {
                            continue;
                        }
                    }
                }

                self.songs_service.set_metadata(
                    selected_song
                        .expect("Can't be a None value !")
                        .get("path")
                        .expect("Can't retrieve path of song file !")
                        .to_string(),
                    &formatted_infos
                );
                // self.next_mode();
            }
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

    fn remove_char_from_input(&mut self) {
        match self.mode.as_str() {
            "modify_popup_songs" => {
                self.input_song_datas[self.song_infos_state.selected().expect("Can't retrieve actual id of selected song !")].1.pop();
            }
            &_ => {}
        }
    }

    fn insert_char_into_input(&mut self, new_char: char) {
        match self.mode.as_str() {
            "modify_popup_songs" => {
                self.input_song_datas[self.song_infos_state.selected().expect("Can't retrieve actual id of selected song !")].1.push_str(&new_char.to_string());
            }
            &_ => {}
        }
    }

    fn set_mode(&mut self, new_mode: PopupState) {
        self.mode_popup = new_mode;
    }

    fn render_popups(&mut self, frame: &mut Frame) {
        let mode = &self.mode_popup;
        match self.active_service {
            ServiceName::PLAYER         => { self.player_service.render_popups(frame, mode); },
            ServiceName::DOWNLOAD       => { self.download_service.render_popups(frame, mode); },
            ServiceName::PLAYLISTS      => { self.playlists_service.render_popups(frame, mode); },
            ServiceName::SONGS          => { self.songs_service.render_popups(frame, mode); },
            _ => {}
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
        let hotkeys_text: &str;
        match self.mode.as_str() {
            "songs" => {
                hotkeys_text = "Navigate <Up/Down> - Play <Enter> - Modify <M> - Like/Unlike <L> - Add to playlist <A> - Delete <Suppr> - Switch Mode <Tab> - Quit <Q>";
            }
            "playing" => {
                hotkeys_text = "Play/Pause <Space> - Previous <Left> - Skip <Right> - Volume <Up/Down> - Shuffle <Backtab> - Loop <T> - Switch Mode <Tab> - Quit <Q>";
            }
            "download" => {
                hotkeys_text = "Navigate <Left/Right> - Download <Enter> - Switch Mode <Tab>";
            }
            "playlists" => {
                hotkeys_text = "Navigate <Up/Down> - Select <Enter> - All songs to queue <Backtab> - New <A> - Modify <M> - Remove <Delete> - Switch Mode <Tab> - Quit <Q>";
            }
            "add_popup_playlists" => {
                hotkeys_text = "Switch Answer <Tab> - Select <Enter> - Close <Esc>";
            }
            "modify_popup_playlists" => {
                hotkeys_text = "Modify <Enter> - Close <Esc>";
            }
            "remove_popup_playlists" => {
                hotkeys_text = "Switch Answer <Tab> - Select <Enter> - Close <Esc>";
            }
            "modify_popup_songs" => {
                hotkeys_text = "Modify <Enter> - Close <Esc>";
            }
            "add_popup_songs" => {
                hotkeys_text = "Navigate <Up/Down> - Add <Enter> - Close <Esc>";
            }
            "remove_popup_songs" => {
                hotkeys_text = "Switch Answer <Tab> - Select <Enter> - Close <Esc>";
            }
            &_ => {
                hotkeys_text = "";
            }
        }
        let hotkeys_section = Block::default()
            .title(Line::from(hotkeys_text).centered());
        frame.render_widget(hotkeys_section, hotkeys);

        // Playlists AND Songs popup (Create/Modify/Delete AND Add)
        // if self.mode.as_str().contains("popup") {
        //     let mut popup_question: String = String::from("");
        //     let mut answers_type: &str = "";
        //     let mut answer_height: Constraint = Constraint::Max(1);

        //     match self.mode.as_str() {
        //         "add_popup_playlists" => {
        //             popup_question = format!("Do you want to add a new playlist ?");
        //             answers_type = "binary";
        //             answer_height = Constraint::Max(1);
        //         }
        //         "modify_popup_playlists" => {
        //             popup_question = format!("What's the new name of the selected playlist ?");
        //             answers_type = "input";
        //             answer_height = Constraint::Max(1);
        //         }
        //         "remove_popup_playlists" => {
        //             popup_question = format!("Do you really want to delete '{}' playlist ?", self.playlists_service.get_all_playlists()[self.playlists_service.get_playlists_state()].playlist_name);
        //             answers_type = "binary";
        //             answer_height = Constraint::Max(1);
        //         }
        //         "modify_popup_songs" => {
        //             popup_question = format!("What is the new informations of this song ?");
        //             answers_type = "inputs_table";
        //             answer_height = Constraint::Max(2);
        //         }
        //         "add_popup_songs" => {
        //             let selected_song = self.songs_service.get_selected_song();
        //             if selected_song.is_some() {
        //                 popup_question = format!(
        //                     "In which playlist(s) do you want to add '{}' song ?",
        //                     selected_song
        //                         .expect("Can't be a None value !")
        //                         .get("title")
        //                         .expect("Can't have an empty title name song !")
        //                 );
        //                 answers_type = "table";
        //                 answer_height = Constraint::Max(3);
        //             }
        //         }
        //         "remove_popup_songs" => {
        //             let selected_song = self.songs_service.get_selected_song();
        //             if selected_song.is_some() {
        //                 popup_question = format!(
        //                     "Do you really want to delete '{}' song ?",
        //                     selected_song
        //                         .expect("Can't be a None value !")
        //                         .get("title")
        //                         .expect("Can't have an empty title name song !")
        //                 );
        //                 answers_type = "binary";
        //                 answer_height = Constraint::Max(1);
        //             }
        //         }
        //         &_ => {
        //             // Delete a random song (1/1000 chance)
        //             popup_question = format!("Bro, no question here. Just you and me. Choose and good luck !");
        //             answers_type = "binary";
        //             answer_height = Constraint::Max(1);
        //         }
        //     }
        //     let popup = frame.area();

        //     let vertical = Layout::vertical([Constraint::Length(12)]).flex(Flex::Center);
        //     let horizontal = Layout::horizontal([Constraint::Length(50)]).flex(Flex::Center);
        //     let [popup] = vertical.areas(popup);
        //     let [popup] = horizontal.areas(popup);

        //     let chunks = Layout::vertical([
        //         Constraint::Max(2),                 // Question's popup
        //         answer_height,                      // Binary answers OR Input text answer OR Selection table
        //     ])
        //     .vertical_margin(3)
        //     .horizontal_margin(8)
        //     .flex(Flex::SpaceBetween)
        //     .split(popup);

        //     frame.render_widget(Clear, popup);

        //     let popup_block = Block::bordered();
        //     frame.render_widget(popup_block, popup);

        //     let popup_question = Line::from(popup_question).alignment(Alignment::Center);
        //     frame.render_widget(Paragraph::new(popup_question).wrap(Wrap { trim: true }), chunks[0]);

        //     match answers_type {
        //         "binary" => {
        //             let positive_answer = "Yes";
        //             let negative_answer = "No";

        //             let answers = Layout::horizontal([
        //                 Constraint::Length(3 + 4),
        //                 Constraint::Length(2 + 4),
        //             ])
        //             .flex(Flex::SpaceBetween)
        //             .split(chunks[1]);

        //             let popup_positive_answer = Line::from(positive_answer).alignment(Alignment::Center);
        //             let popup_negative_answer = Line::from(negative_answer).alignment(Alignment::Center);
        //             let popup_positive_answer_style = if self.is_answer_positive {Color::Magenta} else {Color::Reset};
        //             let popup_negative_answer_style = if !self.is_answer_positive {Color::Magenta} else {Color::Reset};

        //             frame.render_widget(Paragraph::new(popup_positive_answer)
        //                 .style(Style::default().bg(popup_positive_answer_style)), answers[0]);
        //             frame.render_widget(Paragraph::new(popup_negative_answer)
        //                 .style(Style::default().bg(popup_negative_answer_style)), answers[1]);
        //         }
        //         "input" => {
        //             let input_modify_playlists_value = self.input_modify_playlists.get_input();
        //             let popup_input_answer = Line::from(input_modify_playlists_value.as_str()).alignment(Alignment::Center);
        //             frame.render_widget(Paragraph::new(popup_input_answer)
        //                 .style(Style::default().bg(Color::Magenta).fg(Color::White)), chunks[1]);
        //         }
        //         "table" => {
        //             let mut playlists_datas: Vec<Row> = Vec::new();
        //             let all_playlists = self.playlists_service.get_all_playlists();
        //             for playlist in &all_playlists {
        //                 let selected_song = self.songs_service.get_selected_song();
        //                 if selected_song.is_some() {
        //                     let is_in_playlist = playlist.songs_list.contains(selected_song.expect("Can't be a None value !").get("path").expect("Can't retrieve path of song file !"));
        //                     let checkbox: &str;
        //                     if is_in_playlist || playlist.playlist_name == "All songs".to_string() {
        //                         checkbox = "[X]";
        //                     } else { checkbox = "[ ]"; }
        //                     playlists_datas.push(Row::new(vec![
        //                         checkbox,
        //                         &playlist.playlist_name
        //                     ]));
        //                 }
        //             }
        //             let selection_table = Table::new(
        //                 playlists_datas,
        //                 [
        //                     Constraint::Length(3),              // Selection box
        //                     Constraint::Fill(1),                // Playlist name
        //                 ])
        //                 .row_highlight_style(Style::default().bg(Color::Magenta).fg(Color::White));
        //             frame.render_stateful_widget(selection_table, chunks[1], &mut self.song_to_playlists_state);
        //         }
        //         "inputs_table" => {
        //             let mut song_datas: Vec<Row> = Vec::new();
        //             for (name, content) in &self.input_song_datas {
        //                 song_datas.push(Row::new(vec![
        //                     Cell::from(format!("{}:", name)), 
        //                     Cell::from(Text::from(content.to_string()).alignment(TableAlignment::Right))
        //                 ]))
        //             }
        //             let modify_table = Table::new(
        //                 song_datas,
        //                 [
        //                     Constraint::Length(8),              // Entitled name
        //                     Constraint::Fill(1),                // Entitled's content
        //                 ])
        //                 .row_highlight_style(Style::default().bg(Color::Magenta).fg(Color::White));
        //             frame.render_stateful_widget(modify_table, chunks[1], &mut self.song_infos_state);
        //         }
        //         &_ => {}
        //     }
        // }
    }

    // Exit the app on key pressed
    fn exit(&mut self) {
        self.is_running = false;
    }
}
