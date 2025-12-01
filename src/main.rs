#![feature(str_split_remainder)]

mod music;
mod service;

use music::Player;
use service::{Service, PlayingService, DownloadingService, PlaylistsService};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout},
    prelude::{Alignment},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    DefaultTerminal, Frame,
};
use rodio::{OutputStreamBuilder, Sink};
use std::io;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut stream_handle = OutputStreamBuilder::open_default_stream().expect("Unable to get OutputStreamBuilder !");
    stream_handle.log_on_drop(false);
    let mut app = App {
        songs_state: TableState::default().with_selected(0),
        song_to_playlists_state: TableState::default().with_selected(0),
        player: Player::new(
            Sink::connect_new(stream_handle.mixer()), 
            Vec::new(), 
            Arc::new(AtomicU32::new(0))
        ),
        playing_service: PlayingService::new(),
        downloading_service: DownloadingService::new(),
        playlists_service: PlaylistsService::new(),
        mode: "songs".to_string(),
        input_modify_playlists: "".to_string(),
        total_downloading_time: 0.0,
        downloading_started: false,
        all_songs: Vec::new(),
        is_running: false,
        is_answer_positive: false,
    };
    let running_app = app.run(&mut terminal).await;
    ratatui::restore();
    running_app
}

pub struct App {
    // Reorganise & reduce this variables (is_running at the end, some variables can maybe get out ...)
    songs_state: TableState,
    song_to_playlists_state: TableState,
    player: Player,
    playing_service: PlayingService,
    downloading_service: DownloadingService,
    playlists_service: PlaylistsService,
    mode: String,
    input_modify_playlists: String,
    total_downloading_time: f64,
    downloading_started: bool,
    all_songs: Vec<HashMap<String, String>>,
    is_running: bool,
    is_answer_positive: bool,
}

impl App {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.is_running = true;
        self.playing_service.playing_infos = self.player.get_current_song_info();
        self.all_songs = self.player.get_all_songs_from_active_playlist(self.playlists_service.get_active_playlist());
        self.playlists_service.set_all_playlists(self.player.get_all_playlists());

        let (sender, mut receiver) = mpsc::channel(1);

        let tick_rate = Duration::from_millis(250);
        let mut tick_time_elapsed = Instant::now();
        let mut time_downloading = Instant::now();

        while self.is_running {
            // Draw TUI
            terminal.draw(|frame| self.draw(frame))?;

            // Do not wait keys events more than 0.25s after render TUI
            let timeout = tick_rate.saturating_sub(tick_time_elapsed.elapsed());
            if event::poll(timeout).expect("Can't check if event::poll during timeout value !") {
                self.handle_events(sender.clone()).await;
            }

            // Update datas each 0.25s (not each frame bc it makes 10x CPU usage)
            if tick_time_elapsed.elapsed() >= tick_rate {
                self.update_datas(&mut time_downloading, &mut receiver).await;
                tick_time_elapsed = Instant::now();
            }
        }
        Ok(())
    }

    // Function to update all datas
    async fn update_datas(&mut self, time_downloading: &mut Instant, receiver: &mut Receiver<f64>) {
        // Update data in playing section
        self.playing_service.playing_infos = self.player.get_current_song_info();

        // Update progress bar during downloading song(s)
        if !receiver.is_empty() {
            *time_downloading = Instant::now();
            self.total_downloading_time = receiver.recv().await.expect("Can't retrieve estimated downloading duration value !");
            self.downloading_service.input_downloading = format!("Estimated total downloading duration : {}s", self.total_downloading_time as u64);
            self.downloading_started = true;
        }
        if self.downloading_started == true {
            self.downloading_service.state_download = time_downloading.elapsed().as_secs() as f64 / self.total_downloading_time;
        }
        if self.downloading_service.state_download >= 0.99 {
            self.downloading_service.state_download = 0.0;
            self.downloading_started = false;
            self.downloading_service.input_downloading = "Download successfull !".to_string();
        }
        
        // Update all songs
        self.all_songs = self.player.get_all_songs_from_active_playlist(self.playlists_service.get_active_playlist());

        // Update all playlists
        self.playlists_service.set_all_playlists(self.player.get_all_playlists());
    }

    // Retrieve keys events
    async fn handle_events(&mut self, sender: Sender<f64>) {
        match event::read().expect("Can't read events !") {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event, sender).await;
            }
            _ => {}
        };
    }

    // Match key event to dedicated function
    // Refactor function calls to remove intermediate function calls (enter_action() using match isn't necessary if the right function is directly call)
    async fn handle_key_event(&mut self, key_event: KeyEvent, sender: Sender<f64>) {
        match self.mode.as_str() {
            "songs" => {
                match key_event.code {
                    KeyCode::Char('q')              => { self.exit(); },
                    KeyCode::Enter                  => { self.enter_action(); },
                    KeyCode::Up                     => { self.previous(); },
                    KeyCode::Down                   => { self.next(); },
                    KeyCode::Right                  => { self.skip_song(); },
                    KeyCode::Char('l')              => { self.set_favorites(); },
                    KeyCode::Char(' ')              => { self.pause_play_song(); },
                    KeyCode::Tab                    => { self.switch_mode(); },
                    KeyCode::Char('a')              => { self.display_popup("add_song"); },
                    _ => {}
                }
            }
            "download" => {
                match key_event.code {
                    KeyCode::Enter                  => { self.download_songs_from_url(self.downloading_service.input_downloading.to_string(), sender).await; },
                    KeyCode::Backspace              => { self.remove_char_from_input(); },
                    KeyCode::Char(to_insert)        => { self.insert_char_into_input(to_insert); },
                    KeyCode::Tab                    => { self.switch_mode(); },
                    _ => {}
                }
            }
            "playlists" => {
                match key_event.code {
                    KeyCode::Char('q')              => { self.exit(); },
                    KeyCode::Enter                  => { self.enter_action(); },
                    KeyCode::Up                     => { self.playlists_service.previous(); },
                    KeyCode::Down                   => { self.playlists_service.next(); },
                    KeyCode::Tab                    => { self.switch_mode(); },
                    KeyCode::Char('a')              => { self.display_popup("add_playlist"); },
                    KeyCode::Char('m')              => { self.display_popup("modify_playlist"); },
                    KeyCode::Delete                 => { self.display_popup("remove_playlist")},
                    _ => {}
                }
            }
            "add_popup_playlists" => {
                match key_event.code {
                    KeyCode::Enter                  => { self.add_or_not_playlist(); },
                    KeyCode::Tab                    => { self.switch_answer(); },
                    KeyCode::Esc                    => { self.next_mode(); }
                    _ => {}
                }
            }
            "modify_popup_playlists" => {
                match key_event.code {
                    KeyCode::Enter                  => { self.modify_playlist(); },
                    KeyCode::Backspace              => { self.remove_char_from_input(); },
                    KeyCode::Char(to_insert)        => { self.insert_char_into_input(to_insert); },
                    KeyCode::Esc                    => { self.next_mode(); }
                    _ => {}
                }
            }
            "remove_popup_playlists" => {
                match key_event.code {
                    KeyCode::Enter                  => { self.remove_or_not_playlist(); },
                    KeyCode::Tab                    => { self.switch_answer(); },
                    KeyCode::Esc                    => { self.next_mode(); }
                    _ => {}
                }
            }
            "add_popup_songs" => {
                match key_event.code {
                    KeyCode::Up                     => { self.previous(); },
                    KeyCode::Down                   => { self.next(); },
                    KeyCode::Enter                  => { self.add_or_remove_song_to_playlist(); },
                    KeyCode::Esc                    => { self.next_mode(); }
                    _ => {}
                }
            }
            &_ => {}
        }
    }

    // Execute appropriate action depending on active mode
    fn enter_action(&mut self) {
        match self.mode.as_str() {
            "playlists" => {
                let playlist_name = &self.playlists_service.get_all_playlists()[self.playlists_service.get_playlists_state()].playlist_name;
                self.playlists_service.set_active_playlist(playlist_name.to_string());
            }
            "songs" => {
                self.add_song_to_queue();
            }
            &_ => {}
        }
    }

    // Add song to the queue on key pressed
    fn add_song_to_queue(&mut self) {
        let i = self.songs_state.selected();
        if i.is_some() {
            let path = self.all_songs[i.expect("Cannot be a None value !")].get("path");
            let path = path.as_deref().expect("Unable to make the variable as ownership !");
            self.player.add_song_to_queue(&path);
        }
    }

    // Select previous one in active table on key pressed
    fn previous(&mut self) {
        match self.mode.as_str() {
            "songs" => {
                let i = match self.songs_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.all_songs.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.songs_state.select(Some(i));
            }
            "add_popup_songs" => {
                let i = match self.song_to_playlists_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.playlists_service.get_all_playlists().len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.song_to_playlists_state.select(Some(i));
            }
            &_ => {}
        }
    }

    // Select next one in active table on key pressed
    fn next(&mut self) {
        match self.mode.as_str() {
            "songs" => {
                let i = match self.songs_state.selected() {
                    Some(i) => {
                        if i >= self.all_songs.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.songs_state.select(Some(i));
            }
            "add_popup_songs" => {
                let i = match self.song_to_playlists_state.selected() {
                    Some(i) => {
                        if i >= self.playlists_service.get_all_playlists().len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.song_to_playlists_state.select(Some(i));
            }
            &_ => {}
        }
    }

    // Skip playing song on key pressed
    fn skip_song(&mut self) {
        if !self.player.empty() {
            self.player.skip_one();
        }
    }

    // Play/Pause song on key pressed
    fn pause_play_song(&mut self) {
        if !self.player.is_paused() {
            self.player.pause();
        } else { self.player.play(); }
    }

    fn switch_mode(&mut self) {
        self.next_mode();
        match self.mode.as_str() {
            "songs" => {
                self.songs_state.select(Some(0));
            }
            "download" => {
                self.songs_state.select(None);
            }
            "playlists" => {}
            &_ => {}
        }
    }

    fn next_mode(&mut self) {
        let next_mode: &str;
        match self.mode.as_str() {
            "songs" => {
                next_mode = "download";
            }
            "download" => {
                next_mode = "playlists";
            }
            "playlists" => {
                next_mode = "songs";
            }
            "add_popup_playlists" => {
                next_mode = "playlists";
            }
            "modify_popup_playlists" => {
                next_mode = "playlists";
            }
            "remove_popup_playlists" => {
                next_mode = "playlists";
            }
            "add_popup_songs" => {
                next_mode = "songs";
            }
            &_ => {
                next_mode = "";
            }
        }

        self.mode = next_mode.to_string();
    }

    fn display_popup(&mut self, keyword: &str) {
        let selected_playlist = &self.playlists_service.get_all_playlists()[self.playlists_service.get_playlists_state()].playlist_name;
        self.is_answer_positive = false;
        match keyword {
            "add_playlist" => {
                self.mode = "add_popup_playlists".to_string();
            }
            "modify_playlist" => {
                if *selected_playlist != "All songs".to_string() && *selected_playlist != "Favorites".to_string() {
                    self.mode = "modify_popup_playlists".to_string();
                    self.input_modify_playlists = selected_playlist.to_string();
                }
            }
            "remove_playlist" => {
                if *selected_playlist != "All songs".to_string() && *selected_playlist != "Favorites".to_string() {
                    self.mode = "remove_popup_playlists".to_string();
                }
            }
            "add_song" => {
                self.mode = "add_popup_songs".to_string();
            }
            &_ => {}
        }
    }

    fn add_or_not_playlist(&mut self) {
        if self.is_answer_positive {
            self.player.add_playlist();
        }
        self.next_mode();
    }

    fn modify_playlist(&mut self) {
        let i = self.playlists_service.get_playlists_state();
        self.player.modify_playlist(i, &self.input_modify_playlists);
        self.next_mode();
    }

    fn remove_or_not_playlist(&mut self) {
        if self.is_answer_positive {
            let i = self.playlists_service.get_playlists_state();
            self.player.remove_playlist(i);
        }
        self.next_mode();
    }

    fn add_or_remove_song_to_playlist(&mut self) {
        let song_to_add = self.all_songs[self.songs_state.selected().expect("Can't retrieve selected songs !")]
            .get("path")
            .expect("Can't retrieve path of the selected song !")
            .to_string();
        let selected_playlist = self.playlists_service.get_all_playlists()[self.song_to_playlists_state.selected().expect("Can't be empty !")]
            .playlist_name
            .to_string();
        if selected_playlist != "All songs".to_string() {
            self.player.add_or_remove_song_to_playlist(song_to_add, selected_playlist);
        }
    }

    fn switch_answer(&mut self) {
        if self.is_answer_positive {
            self.is_answer_positive = false;
        } else { self.is_answer_positive = true };
    }

    fn set_favorites(&mut self) {
        let i = self.songs_state.selected();
        if i.is_some() {
            let path = self.all_songs[i.expect("Cannot be a None value !")].get("path");
            let path = path.as_deref().expect("Unable to make the varibale as ownership !");
            self.player.set_favorites(&path);
        }
    }

    fn remove_char_from_input(&mut self) {
        match self.mode.as_str() {
            "download" => {
                self.downloading_service.input_downloading.pop();
            }
            "modify_popup_playlists" => {
                self.input_modify_playlists.pop();
            }
            &_ => {}
        }
    }

    fn insert_char_into_input(&mut self, new_char: char) {
        match self.mode.as_str() {
            "download" => {
                self.downloading_service.input_downloading.push_str(&new_char.to_string());
            }
            "modify_popup_playlists" => {
                if self.input_modify_playlists.len() < 20 {
                    self.input_modify_playlists.push_str(&new_char.to_string());
                }
            }
            &_ => {}
        }
    }

    async fn download_songs_from_url(&mut self, url: String, sender: Sender<f64>) {
        self.downloading_service.input_downloading = "Starting to fetch datas from YouTube URL ...".to_string();
        tokio::spawn( async move {
            let (urls, duration) = music::retrieve_songs_datas_from(&url).await;
            sender.send(duration).await.expect("Can't send estimated downloading time value !");
            for song_url in urls {
                music::download_song(song_url).await;
            }
        });
    }

    // Convert seconds to minutes/seconds
    fn seconds_to_minsec(seconds: f64) -> (u32, u32) {
        let min = (seconds / 60.0).floor() as u32;
        let sec = (seconds % 60.0).round() as u32;

        (min, sec)
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
        self.playing_service.render(frame, playing);

        // Downloading section
        self.downloading_service.render(frame, download);

        // Playlists & Songs section
        let horizontal = Layout::horizontal([
            Constraint::Fill(1),              // Playlists section
            Constraint::Fill(4),              // Songs section
        ]);
        let [playlists, songs] = horizontal.areas(playlists_songs);

        /*// Playlists section
        let mut playlists_datas: Vec<Row> = Vec::new();
        for playlist in &self.all_playlists {
            playlists_datas.push(Row::new(vec![
                playlist.playlist_name.as_str(),
            ]));
        }
        let playlists_border_style = if self.mode.as_str() == "playlists" {Color::Magenta} else {Color::Reset};
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
        frame.render_stateful_widget(playlists_table, playlists, &mut self.playlists_state);*/
        self.playlists_service.render(frame, playlists);

        // Songs section
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
        let songs_border_style = if self.mode.as_str() == "songs" {Color::Magenta} else {Color::Reset};
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
        frame.render_stateful_widget(songs_table, songs, &mut self.songs_state);

        // Hotkeys section
        let hotkeys_text: &str;
        match self.mode.as_str() {
            "songs" => {
                hotkeys_text = "Navigate <Up/Down> - Play <Enter> - Play/Pause <Space> - Like/Unlike <L> - Skip <Right> - Switch Mode <Tab> - Quit <Q>";
            }
            "download" => {
                hotkeys_text = "Download <Enter> - Switch Mode <Tab>";
            }
            "playlists" => {
                hotkeys_text = "Navigate <Up/Down> - Select <Enter> - New <A> - Modify <M> - Remove <Delete> - Switch Mode <Tab> - Quit <Q>";
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
            "add_popup_songs" => {
                hotkeys_text = "Navigate <Up/Down> - Add <Enter> - Close <Esc>";
            }
            &_ => {
                hotkeys_text = "";
            }
        }
        let hotkeys_section = Block::default()
            .title(Line::from(hotkeys_text).centered());
        frame.render_widget(hotkeys_section, hotkeys);

        // Playlists AND Songs popup (Create/Modify/Delete AND Add)
        if self.mode.as_str().contains("popup") {
            let popup_question: String;
            let answers_type: &str;
            let answer_height: Constraint;

            match self.mode.as_str() {
                "add_popup_playlists" => {
                    popup_question = format!("Do you want to add a new playlist ?");
                    answers_type = "binary";
                    answer_height = Constraint::Max(1);
                }
                "modify_popup_playlists" => {
                    popup_question = format!("What's the new name of the selected playlist ?");
                    answers_type = "input";
                    answer_height = Constraint::Max(1);
                }
                "remove_popup_playlists" => {
                    popup_question = format!("Do you really want to delete '{}' playlist ?", self.playlists_service.get_all_playlists()[self.playlists_service.get_playlists_state()].playlist_name);
                    answers_type = "binary";
                    answer_height = Constraint::Max(1);
                }
                "add_popup_songs" => {
                    popup_question = format!("In which playlist(s) do you want to add '{}' song ?", self.all_songs[self.songs_state.selected().expect("Can't be empty !")].get("title").expect("Can't have an empty title name song !"));
                    answers_type = "table";
                    answer_height = Constraint::Max(3);
                }
                &_ => {
                    // Delete a random song (1/1000 chance)
                    popup_question = format!("Bro, no question here. Just you and me. Choose and good luck !");
                    answers_type = "binary";
                    answer_height = Constraint::Max(1);
                }
            }
            let popup = frame.area();

            let vertical = Layout::vertical([Constraint::Length(12)]).flex(Flex::Center);
            let horizontal = Layout::horizontal([Constraint::Length(50)]).flex(Flex::Center);
            let [popup] = vertical.areas(popup);
            let [popup] = horizontal.areas(popup);

            let chunks = Layout::vertical([
                Constraint::Max(2),                 // Question's popup
                answer_height,                      // Binary answers OR Input text answer OR Selection table
            ])
            .vertical_margin(3)
            .horizontal_margin(8)
            .flex(Flex::SpaceBetween)
            .split(popup);

            frame.render_widget(Clear, popup);

            let popup_block = Block::bordered();
            frame.render_widget(popup_block, popup);

            let popup_question = Line::from(popup_question).alignment(Alignment::Center);
            frame.render_widget(Paragraph::new(popup_question).wrap(Wrap { trim: true }), chunks[0]);

            match answers_type {
                "binary" => {
                    let positive_answer = "Yes";
                    let negative_answer = "No";

                    let answers = Layout::horizontal([
                        Constraint::Length(3 + 4),
                        Constraint::Length(2 + 4),
                    ])
                    .flex(Flex::SpaceBetween)
                    .split(chunks[1]);

                    let popup_positive_answer = Line::from(positive_answer).alignment(Alignment::Center);
                    let popup_negative_answer = Line::from(negative_answer).alignment(Alignment::Center);
                    let popup_positive_answer_style = if self.is_answer_positive {Color::Magenta} else {Color::Reset};
                    let popup_negative_answer_style = if !self.is_answer_positive {Color::Magenta} else {Color::Reset};

                    frame.render_widget(Paragraph::new(popup_positive_answer)
                        .style(Style::default().bg(popup_positive_answer_style)), answers[0]);
                    frame.render_widget(Paragraph::new(popup_negative_answer)
                        .style(Style::default().bg(popup_negative_answer_style)), answers[1]);
                }
                "input" => {
                    let popup_input_answer = Line::from(self.input_modify_playlists.as_str()).alignment(Alignment::Center);
                    frame.render_widget(Paragraph::new(popup_input_answer)
                        .style(Style::default().bg(Color::Magenta).fg(Color::White)), chunks[1]);
                }
                "table" => {
                    let mut playlists_datas: Vec<Row> = Vec::new();
                    for playlist in self.playlists_service.get_all_playlists() {
                        let is_in_playlist = playlist.songs_list.contains(self.all_songs[self.songs_state.selected().expect("Can't be empty !")].get("path").expect("Can't have an empty title name song !"));
                        let checkbox: &str;
                        if is_in_playlist || playlist.playlist_name == "All songs".to_string() {
                            checkbox = "[X]";
                        } else { checkbox = "[ ]"; }
                        playlists_datas.push(Row::new(vec![
                            checkbox,
                            &playlist.playlist_name
                        ]));
                    }
                    let selection_table = Table::new(
                        playlists_datas,
                        [
                            Constraint::Length(3),              // Selection box
                            Constraint::Fill(1),                // Playlist name
                        ])
                        .row_highlight_style(Style::default().bg(Color::Magenta).fg(Color::White));
                    frame.render_stateful_widget(selection_table, chunks[1], &mut self.song_to_playlists_state);
                }
                &_ => {}
            }
        }
    }

    // Exit the app on key pressed
    fn exit(&mut self) {
        self.is_running = false;
    }
}
