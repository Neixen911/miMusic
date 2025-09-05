#![feature(str_split_remainder)]

mod music;

use music::Player;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout},
    prelude::{Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Cell, Clear, Gauge, Paragraph, Row, Table, TableState, Wrap},
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
        playlists_state: TableState::default().with_selected(0),
        player: Player::new(
            Sink::connect_new(stream_handle.mixer()), 
            Vec::new(), 
            Arc::new(AtomicU32::new(0))
        ),
        playing_infos: Vec::new(),
        mode: "songs".to_string(),
        input_editing: "ex: https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
        total_downloading_time: 0.0,
        state_download: 0.0,
        downloading_started: false,
        all_songs: Vec::new(),
        all_playlists: Vec::new(),
        active_playlist: "All songs".to_string(),
        is_running: false,
    };
    let running_app = app.run(&mut terminal).await;
    ratatui::restore();
    running_app
}

pub struct App {
    // Reorganise & reduce this variables (is_running at the end, some variables can maybe get out ...)
    songs_state: TableState,
    playlists_state: TableState,
    player: Player,
    playing_infos: Vec<String>,
    mode: String,
    input_editing: String,
    total_downloading_time: f64,
    state_download: f64,
    downloading_started: bool,
    all_songs: Vec<HashMap<String, String>>,
    all_playlists: Vec<String>,
    active_playlist: String,
    is_running: bool,
}

impl App {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.is_running = true;
        self.playing_infos = self.player.get_current_song_info();
        self.all_songs = self.player.get_all_songs_from_active_playlist(&self.active_playlist);
        self.all_playlists = self.player.get_all_playlists();

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
        self.playing_infos = self.player.get_current_song_info();

        // Update progress bar during downloading song(s)
        if !receiver.is_empty() {
            *time_downloading = Instant::now();
            self.total_downloading_time = receiver.recv().await.expect("Can't retrieve estimated downloading duration value !");
            self.input_editing = format!("Estimated total downloading duration : {}s", self.total_downloading_time as u64);
            self.downloading_started = true;
        }
        if self.downloading_started == true {
            self.state_download = time_downloading.elapsed().as_secs() as f64 / self.total_downloading_time;
        }
        if self.state_download >= 0.99 {
            self.state_download = 0.0;
            self.downloading_started = false;
            self.input_editing = "Download successfull !".to_string();
        }
        
        // Update all songs
        self.all_songs = self.player.get_all_songs_from_active_playlist(&self.active_playlist);

        // Update all playlists
        self.all_playlists = self.player.get_all_playlists();
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
                    _ => {}
                }
            }
            "download" => {
                match key_event.code {
                    KeyCode::Enter                  => { self.download_songs_from_url(self.input_editing.to_string(), sender).await; },
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
                    KeyCode::Up                     => { self.previous(); },
                    KeyCode::Down                   => { self.next(); },
                    KeyCode::Tab                    => { self.switch_mode(); },
                    KeyCode::Delete                 => { self.remove_popup_playlists(); },
                    _ => {}
                }
            }
            "popup_playlists" => {
                match key_event.code {
                    KeyCode::Enter                  => { self.remove_or_not_playlist(); },
                    KeyCode::Tab                    => { self.switch_answer(); },
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
                self.set_active_playlist();
            }
            "songs" => {
                self.add_song_to_queue();
            }
            &_ => {}
        }
    }

    // Set the new active playlist
    fn set_active_playlist(&mut self) {
        self.active_playlist = self.all_playlists[self.playlists_state.selected().expect("Can't retrieve active playlist selected id !")].to_string();
    }

    // Add song to the queue on key pressed
    fn add_song_to_queue(&mut self) {
        let i = self.songs_state.selected();
        if i.is_some() {
            let path = self.all_songs[i.expect("Cannot be a None value !")].get("path");
            let path = path.as_deref().expect("Unable to make the varibale as ownership !");
            self.player.add_song_to_queue(&path);
        }
    }

    // Select previous one in active table on key pressed
    fn previous(&mut self) {
        match self.mode.as_str() {
            "playlists" => {
                let i = match self.playlists_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.all_playlists.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.playlists_state.select(Some(i));
            }
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
            &_ => {}
        }
    }

    // Select next one in active table on key pressed
    fn next(&mut self) {
        match self.mode.as_str() {
            "playlists" => {
                let i = match self.playlists_state.selected() {
                    Some(i) => {
                        if i >= self.all_playlists.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.playlists_state.select(Some(i));
            }
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
        self.mode = self.next_mode();
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

    fn next_mode(&mut self) -> String {
        match self.mode.as_str() {
            "songs" => {
                return "download".to_string()
            }
            "download" => {
                return "playlists".to_string()
            }
            "playlists" => {
                return "songs".to_string()
            }
            "popup_playlists" => {
                return "playlists".to_string()
            }
            &_ => {}
        }

        return "".to_string()
    }

    fn remove_popup_playlists(&mut self) {
        self.mode = "popup_playlists".to_string();
    }

    fn remove_or_not_playlist(&mut self) {
        self.mode = self.next_mode();
    }

    fn switch_answer(&mut self) {

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
        self.input_editing.pop();
    }

    fn insert_char_into_input(&mut self, new_char: char) {
        self.input_editing.push_str(&new_char.to_string());
    }

    async fn download_songs_from_url(&mut self, url: String, sender: Sender<f64>) {
        self.input_editing = "Starting to fetch datas from YouTube URL ...".to_string();
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
        let chunks = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(playing);

        let playing_section = Block::default()
            .title(Line::from("Now Playing"))
            .borders(ratatui::widgets::Borders::ALL);
        frame.render_widget(playing_section, playing);

        let mut playing_lines: Vec<Line> = Vec::new();
        playing_lines.push(Line::from(self.playing_infos.get(0).expect("Unable to get title from current playing song !").to_string()));
        playing_lines.push(Line::from(self.playing_infos.get(1).expect("Unable to get artist from current playing song !").to_string()));
        let infos_section = Paragraph::new(playing_lines);
        frame.render_widget(infos_section, chunks[0]);

        let act_duration_song = self.playing_infos.get(2)
            .expect("Unable to get current duration from current playing song !")
            .to_string()
            .parse::<f64>()
            .expect("Unable to convert into f64 !");
        let max_duration_song = self.playing_infos.get(3)
            .expect("Unable to get maximum duration from current playing song !")
            .to_string()
            .parse::<f64>()
            .expect("Unable to convert into f64 !");
        let mut ratio = 0.0;
        let (act_minutes, act_seconds) = Self::seconds_to_minsec(act_duration_song);
        let (max_minutes, max_seconds) = Self::seconds_to_minsec(max_duration_song);
        let playing_label = Span::styled(
            format!("{:02}", act_minutes) 
            + ":" 
            + format!("{:02}", act_seconds).as_str() 
            + " / " 
            + format!("{:02}", max_minutes).as_str() 
            + ":" 
            + format!("{:02}", max_seconds).as_str(),
            Style::default(),
        );
        if max_duration_song != 0.0 {
            ratio = act_duration_song / max_duration_song;
        }
        let playing_gauge_section = Gauge::default()
            .ratio(ratio)
            .gauge_style(Color::Magenta)
            .label(playing_label);
        frame.render_widget(playing_gauge_section, chunks[1]);

        // Download section
        let chunks = Layout::vertical([
            Constraint::Length(3),
        ])
        .margin(1)
        .split(download);
        
        let download_border_style = if self.mode.as_str() == "download" {Color::Magenta} else {Color::Reset};
        let downloading_section = Block::default()
            .title(Line::from("Download URL"))
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(download_border_style);
        frame.render_widget(downloading_section, download);

        let downloading_label = Span::styled(
            &self.input_editing,
            Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC),
        );
        let downloading_gauge_section = Gauge::default()
            .ratio(self.state_download)
            .gauge_style(Color::Magenta)
            .label(downloading_label);
        frame.render_widget(downloading_gauge_section, chunks[0]);

        // Playlists & Songs section
        let horizontal = Layout::horizontal([
            Constraint::Fill(1),              // Playlists section
            Constraint::Fill(4),              // Songs section
        ]);
        let [playlists, songs] = horizontal.areas(playlists_songs);

        // Playlists section
        let mut playlists_datas: Vec<Row> = Vec::new();
        for playlist in &self.all_playlists {
            playlists_datas.push(Row::new(vec![
                playlist.to_string()
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
        frame.render_stateful_widget(playlists_table, playlists, &mut self.playlists_state);

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
                Constraint::Fill(2),
                Constraint::Fill(1),
                Constraint::Max(10),
                Constraint::Max(10),
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
        let mut hotkeys_text = "";
        match self.mode.as_str() {
            "songs" => {
                hotkeys_text = "Navigate <Up/Down> - Play <Enter> - Play/Pause <Space> - Like/Unlike <L> - Skip <Right> - Switch Mode <Tab> - Quit <Q>";
            }
            "download" => {
                hotkeys_text = "Download <Enter> - Switch Mode <Tab>";
            }
            "playlists" => {
                hotkeys_text = "Navigate <Up/Down> - Select <Enter> - Remove <Delete> - Switch Mode <Tab> - Quit <Q>";
            }
            "popup_playlists" => {
                hotkeys_text = "Select <Enter> - Switch Answer <Tab>";
            }
            &_ => {}
        }
        let hotkeys_section = Block::default()
            .title(Line::from(hotkeys_text).centered());
        frame.render_widget(hotkeys_section, hotkeys);

        // Playlists popup (Create/Modify/Delete)
        if self.mode.as_str() == "popup_playlists" {
            let playlists_popup = frame.area();

            let vertical = Layout::vertical([Constraint::Length(12)]).flex(Flex::Center);
            let horizontal = Layout::horizontal([Constraint::Length(50)]).flex(Flex::Center);
            let [playlists_popup] = vertical.areas(playlists_popup);
            let [playlists_popup] = horizontal.areas(playlists_popup);

            let chunks = Layout::vertical([
                Constraint::Length(2),
                Constraint::Max(3),
                Constraint::Length(1),
            ])
            .vertical_margin(3)
            .horizontal_margin(8)
            .split(playlists_popup);

            let answers = Layout::horizontal([
                Constraint::Min(1),
                Constraint::Min(1),
            ])
            .split(chunks[2]);

            frame.render_widget(Clear, playlists_popup);

            let playlists_popup_block = Block::bordered();
            frame.render_widget(playlists_popup_block, playlists_popup);

            let playlists_popup_question = Line::from("Do you really want to remove '".to_owned() + &self.active_playlist + "' playlist ?").alignment(Alignment::Center);
            frame.render_widget(Paragraph::new(playlists_popup_question).wrap(Wrap { trim: true }), chunks[0]);

            let playlists_popup_answer_positive = Line::from("Yes").alignment(Alignment::Left);
            frame.render_widget(Paragraph::new(playlists_popup_answer_positive), answers[0]);
            let playlists_popup_answer_negative = Line::from("No").alignment(Alignment::Right);
            frame.render_widget(Paragraph::new(playlists_popup_answer_negative), answers[1]);
        }
    }

    // Exit the app on key pressed
    fn exit(&mut self) {
        self.is_running = false;
    }
}
