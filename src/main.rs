#![feature(str_split_remainder)]

mod music;
mod service;

use music::Player;
use service::{Service, PlayingService, DownloadingService, PlaylistsService, SongsService};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout},
    prelude::{Alignment},
    style::{Color, Style},
    text::{Line},
    widgets::{Block, Clear, Paragraph, Row, Table, TableState, Wrap},
    DefaultTerminal, Frame,
};
use rodio::{OutputStreamBuilder, Sink};
use std::io;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;
use tokio::sync::watch::{self, Receiver, Sender};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut stream_handle = OutputStreamBuilder::open_default_stream().expect("Unable to get OutputStreamBuilder !");
    stream_handle.log_on_drop(false);
    let mut app = App {
        song_to_playlists_state: TableState::default().with_selected(0),
        song_infos_state: TableState::default().with_selected(0),
        input_song_datas: Vec::new(),
        player: Player::new(
            Sink::connect_new(stream_handle.mixer()), 
            Vec::new(), 
            Arc::new(AtomicU32::new(0))
        ),
        playing_service: PlayingService::new(),
        downloading_service: DownloadingService::new(),
        playlists_service: PlaylistsService::new(),
        songs_service: SongsService::new(),
        mode: "songs".to_string(),
        input_modify_playlists: "".to_string(),
        downloading_started: false,
        is_running: false,
        is_answer_positive: false,
    };
    let running_app = app.run(&mut terminal).await;
    ratatui::restore();
    running_app
}

pub struct App {
    // Reorganise & reduce this variables (is_running at the end, some variables can maybe get out ...)
    song_to_playlists_state: TableState,
    song_infos_state: TableState,
    input_song_datas: Vec<(String, String)>,
    player: Player,
    playing_service: PlayingService,
    downloading_service: DownloadingService,
    playlists_service: PlaylistsService,
    songs_service: SongsService,
    mode: String,
    input_modify_playlists: String,
    downloading_started: bool,
    is_running: bool,
    is_answer_positive: bool,
}

impl App {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.is_running = true;
        self.playing_service.playing_infos = self.player.get_current_song_info();
        self.songs_service.set_all_songs(self.player.get_all_songs_from_active_playlist(self.playlists_service.get_active_playlist()));
        self.playlists_service.set_all_playlists(self.player.get_all_playlists());

        let (sender, mut receiver) = watch::channel((0, 0, 0.0));

        let tick_rate = Duration::from_millis(250);
        let mut tick_time_elapsed = Instant::now();

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
                self.update_datas(&mut receiver).await;
                tick_time_elapsed = Instant::now();
            }
        }
        Ok(())
    }

    // Function to update all datas
    async fn update_datas(&mut self, receiver: &mut Receiver<(u32, u32, f64)>) {
        // Update data in playing section
        self.playing_service.playing_infos = self.player.get_current_song_info();

        // Update progress bar during downloading song(s)
        if self.downloading_started == true {
            let (downloading_index, downloading_total, downlading_percent) = *receiver.borrow();
            let mut index_of_total = String::from("");
            if downloading_index != 0 && downloading_total != 0 {
                index_of_total = format!("{}/{}", downloading_index, downloading_total);
            }
            if downlading_percent > 0.0 && downlading_percent <= 99.0 {
                self.downloading_service.state_download = downlading_percent / 100.0;
                self.downloading_service.input_downloading = format!("Download {}: {}%", index_of_total, downlading_percent);
            } else if self.downloading_service.state_download > 0.0 && downlading_percent < 0.0 {
                self.downloading_service.state_download = 0.0;
                self.downloading_started = false;
                self.downloading_service.input_downloading = "Download successfull !".to_string();
            }
        }
        
        // Update all songs
        self.songs_service.set_all_songs(self.player.get_all_songs_from_active_playlist(self.playlists_service.get_active_playlist()));

        // Update all playlists
        self.playlists_service.set_all_playlists(self.player.get_all_playlists());
    }

    // Retrieve keys events
    async fn handle_events(&mut self, sender: Sender<(u32, u32, f64)>) {
        match event::read().expect("Can't read events !") {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event, sender).await;
            }
            _ => {}
        };
    }

    // Match key event to dedicated function
    // Refactor function calls to remove intermediate function calls (enter_action() using match isn't necessary if the right function is directly call)
    async fn handle_key_event(&mut self, key_event: KeyEvent, sender: Sender<(u32, u32, f64)>) {
        match self.mode.as_str() {
            "songs" => {
                match key_event.code {
                    KeyCode::Char('q')              => { self.exit(); },
                    KeyCode::Enter                  => { self.enter_action(); },
                    KeyCode::Up                     => { self.songs_service.previous(); },
                    KeyCode::Down                   => { self.songs_service.next(); },
                    KeyCode::Right                  => { self.skip_song(); },
                    KeyCode::Char('m')              => { self.display_popup("modify_song") },
                    KeyCode::Char('l')              => { self.set_favorites(); },
                    KeyCode::Char(' ')              => { self.pause_play_song(); },
                    KeyCode::Tab                    => { self.switch_mode(); },
                    KeyCode::Char('a')              => { self.display_popup("add_song"); },
                    KeyCode::Delete                 => { self.display_popup("remove_song"); },
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
                    KeyCode::Delete                 => { self.display_popup("remove_playlist"); },
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
            "modify_popup_songs" => {
                match key_event.code {
                    KeyCode::Enter                  => { self.modify_song(); },
                    KeyCode::Backspace              => { self.remove_char_from_input(); },
                    KeyCode::Char(to_insert)        => { self.insert_char_into_input(to_insert); },
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
            "remove_popup_songs" => {
                match key_event.code {
                    KeyCode::Enter                  => { self.remove_or_not_song(); },
                    KeyCode::Tab                    => { self.switch_answer(); },
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
        let i = self.songs_service.get_songs_state();
        if i.is_some() {
            let path = self.songs_service.get_all_songs()[i.expect("Cannot be a None value !")].get("path");
            let path = path.as_deref().expect("Unable to make the variable as ownership !");
            self.player.add_song_to_queue(&path);
        }
    }

    // Select previous one in active table on key pressed
    fn previous(&mut self) {
        match self.mode.as_str() {
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
            "modify_popup_songs" => {
                let i = match self.song_infos_state.selected() {
                    Some(i) => {
                        if i >= self.songs_service.get_modify_song_infos().len() - 1 {
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
                self.songs_service.set_songs_state(Some(0));
            }
            "download" => {
                self.songs_service.set_songs_state(None);
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
            "modify_popup_songs" => {
                next_mode = "songs"
            }
            "add_popup_songs" => {
                next_mode = "songs";
            }
            "remove_popup_songs" => {
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
                    self.input_modify_playlists = selected_playlist.to_string();
                    self.mode = "modify_popup_playlists".to_string();
                }
            }
            "remove_playlist" => {
                if *selected_playlist != "All songs".to_string() && *selected_playlist != "Favorites".to_string() {
                    self.mode = "remove_popup_playlists".to_string();
                }
            }
            "modify_song" => {
                let modify_song_infos = self.songs_service.get_modify_song_infos();
                self.input_song_datas = Vec::new();
                for (entitled_name, entitled_content) in modify_song_infos {
                    let entitled_name_formatted = entitled_name;
                    let entitled_content_formatted = entitled_content;
                    self.input_song_datas.push((entitled_name_formatted, entitled_content_formatted));
                }
                self.song_infos_state.select(Some(0));
                self.mode = "modify_popup_songs".to_string();
            }
            "add_song" => {
                self.mode = "add_popup_songs".to_string();
            }
            "remove_song" => {
                self.mode = "remove_popup_songs".to_string();
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

    fn modify_song(&mut self) {
        if self.song_infos_state.selected().expect("Can't retrieve actual id of selected song !") < self.songs_service.get_modify_song_infos().len() - 1 {
            self.next();
        } else {
            self.songs_service.modifying_metadata(&self.input_song_datas);
            self.next_mode();
        }
    }

    fn remove_or_not_playlist(&mut self) {
        if self.is_answer_positive {
            let i = self.playlists_service.get_playlists_state();
            self.player.remove_playlist(i);
        }
        self.next_mode();
    }

    fn add_or_remove_song_to_playlist(&mut self) {
        let song_to_add = self.songs_service.get_all_songs()[self.songs_service.get_songs_state().expect("Can't retrieve active song id !")]
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

    fn remove_or_not_song(&mut self) {
        if self.is_answer_positive {
            let song_to_remove = self.songs_service.get_all_songs()[self.songs_service.get_songs_state().expect("Can't retrieve active song id !")]
                .get("path")
                .expect("Can't retrieve path of the selected song !")
                .to_string();
            self.player.remove_song(song_to_remove);
        }
        self.next_mode();
    }

    fn switch_answer(&mut self) {
        if self.is_answer_positive {
            self.is_answer_positive = false;
        } else { self.is_answer_positive = true };
    }

    fn set_favorites(&mut self) {
        let i = self.songs_service.get_songs_state();
        if i.is_some() {
            let path = self.songs_service.get_all_songs()[i.expect("Cannot be a None value !")].get("path");
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
            "modify_popup_songs" => {
                self.input_song_datas[self.song_infos_state.selected().expect("Can't retrieve actual id of selected song !")].1.pop();
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
            "modify_popup_songs" => {
                self.input_song_datas[self.song_infos_state.selected().expect("Can't retrieve actual id of selected song !")].1.push_str(&new_char.to_string());
            }
            &_ => {}
        }
    }

    async fn download_songs_from_url(&mut self, url: String, sender: Sender<(u32, u32, f64)>) {
        self.downloading_service.input_downloading = "Starting to fetch datas from YouTube URL ...".to_string();
        self.downloading_started = true;
        tokio::spawn( async move {
            music::download_song(sender, url).await;
        });
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

        self.playlists_service.render(frame, playlists);
        self.songs_service.render(frame, songs);

        // Hotkeys section
        let hotkeys_text: &str;
        match self.mode.as_str() {
            "songs" => {
                hotkeys_text = "Navigate <Up/Down> - Play <Enter> - Play/Pause <Space> - Modify <M> - Like/Unlike <L> - Add to playlist <A> - Delete <Suppr> - Skip <Right> - Switch Mode <Tab> - Quit <Q>";
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
            "modify_popup_songs" => {
                hotkeys_text = "Modify <Enter> - Close <Esc>"
            }
            "add_popup_songs" => {
                hotkeys_text = "Navigate <Up/Down> - Add <Enter> - Close <Esc>";
            }
            "remove_popup_songs" => {
                hotkeys_text = "Switch Answer <Tab> - Select <Enter> - Close <Esc>"
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
                "modify_popup_songs" => {
                    popup_question = format!("What is the new informations of this song ?");
                    answers_type = "inputs_table";
                    answer_height = Constraint::Max(2);
                }
                "add_popup_songs" => {
                    popup_question = format!("In which playlist(s) do you want to add '{}' song ?", self.songs_service.get_all_songs()[self.songs_service.get_songs_state().expect("Can't retrieve active song id !")].get("title").expect("Can't have an empty title name song !"));
                    answers_type = "table";
                    answer_height = Constraint::Max(3);
                }
                "remove_popup_songs" => {
                    popup_question = format!("Do you really want to delete '{}' song ?", self.songs_service.get_all_songs()[self.songs_service.get_songs_state().expect("Can't retrieve active song id !")].get("title").expect("Can't have an empty title name song !"));
                    answers_type = "binary";
                    answer_height = Constraint::Max(1);
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
                        let is_in_playlist = playlist.songs_list.contains(self.songs_service.get_all_songs()[self.songs_service.get_songs_state().expect("Can't retrieve active song id !")].get("path").expect("Can't retrieve path of song file !"));
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
                "inputs_table" => {
                    let mut song_datas: Vec<Row> = Vec::new();
                    for (name, content) in &self.input_song_datas {
                        song_datas.push(Row::new(vec![
                            format!("{}:", name), 
                            content.to_string()
                        ]))
                    }
                    let modify_table = Table::new(
                        song_datas,
                        [
                            Constraint::Length(8),              // Entitled name
                            Constraint::Fill(1),                // Entitled's content
                        ])
                        .row_highlight_style(Style::default().bg(Color::Magenta).fg(Color::White));
                    frame.render_stateful_widget(modify_table, chunks[1], &mut self.song_infos_state);
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
