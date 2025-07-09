#![feature(str_split_remainder)]

use std::io;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
use rodio::{OutputStream, Sink};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Style},
    text::{Line, Text},
    widgets::{Block, Gauge, Paragraph, Row, Table, TableState},
    DefaultTerminal, Frame,
};

mod music;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug, Default)]
pub struct App {
    state_table: TableState,
    player: music::Player,
    playing_infos: Vec<String>,
    all_songs: Vec<HashMap<String, String>>,
    is_running: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // Initialisation of App's variables
        self.is_running = true;
        self.state_table = TableState::default().with_selected(0);
        self.player = music::Player { m_song_infos: Vec::new(), end_of_song_signal: Arc::new(AtomicU32::new(0)) };
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        self.all_songs = music::get_all_songs();
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();
        self.on_tick(&sink);

        while self.is_running {
            // Draw TUI
            terminal.draw(|frame| self.draw(frame))?;

            // Detect keys events
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                self.handle_events(&sink)?;
            }

            // Loop to updated data
            if last_tick.elapsed() >= tick_rate {
                self.on_tick(&sink);
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    // Function to retrieve data from playing section
    fn on_tick(&mut self, sink: &Sink) {
        self.playing_infos = music::get_current_song_info(sink, &mut self.player);
    }

    // Retrieve keys events
    fn handle_events(&mut self, sink: &Sink) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event, sink)
            }
            _ => {}
        };
        Ok(())
    }

    // Match key event to dedicated function
    fn handle_key_event(&mut self, key_event: KeyEvent, sink: &Sink) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Enter => self.add_song_to_queue(sink),
            KeyCode::Up => self.previous_song(),
            KeyCode::Down => self.next_song(),
            KeyCode::Right => self.skip_song(sink),
            KeyCode::Char(' ') => self.pause_play_song(sink),
            _ => {}
        }
    }

    // Add song to the queue on key pressed
    fn add_song_to_queue(&mut self, sink: &Sink) {
        let i = match self.state_table.selected() {
            Some(i) => {
                i
            }
            None => 0,
        };
        let path = self.all_songs[i].get("path");
        let path = path.as_deref().unwrap();
        self.player.m_song_infos.push(music::get_song_infos_from_file(&path)); // To optimise
        music::add_song_to_queue(sink, &path, &mut self.player);
    }

    // Select previous song in table on key pressed
    fn previous_song(&mut self) {
        let i = match self.state_table.selected() {
            Some(i) => {
                if i == 0 {
                    self.all_songs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state_table.select(Some(i));
    }

    // Select next song in table on key pressed
    fn next_song(&mut self) {
        let i = match self.state_table.selected() {
            Some(i) => {
                if i >= self.all_songs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state_table.select(Some(i));
    }

    // Skip playing song on key pressed
    fn skip_song(&mut self, sink: &Sink) {
        if sink.len() > 0 {
            sink.skip_one();
        }
    }

    // Play/Pause song on key pressed
    fn pause_play_song(&mut self, sink: &Sink) {
        if !sink.is_paused() {
            sink.pause();
        } else { sink.play(); }
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
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(2),
        ]);
        let [app, playing, songs, hotkeys] = vertical.areas(frame.area());

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
        playing_lines.push(Line::from(self.playing_infos.get(0).unwrap().to_string()));
        playing_lines.push(Line::from(self.playing_infos.get(1).unwrap().to_string()));
        let infos_section = Paragraph::new(playing_lines);
        frame.render_widget(infos_section, chunks[0]);

        let act_duration_song = self.playing_infos.get(2).unwrap().to_string().parse::<f64>().unwrap();
        let max_duration_song = self.playing_infos.get(3).unwrap().to_string().parse::<f64>().unwrap();
        let mut ratio = 0.0;
        let (act_minutes, act_seconds) = Self::seconds_to_minsec(act_duration_song);
        let (max_minutes, max_seconds) = Self::seconds_to_minsec(max_duration_song);
        let label = format!("{:02}", act_minutes) 
            + ":" 
            + format!("{:02}", act_seconds).as_str() 
            + " / " 
            + format!("{:02}", max_minutes).as_str() 
            + ":" 
            + format!("{:02}", max_seconds).as_str();
        if max_duration_song != 0.0 {
            ratio = act_duration_song / max_duration_song;
        }
        let gauge_section = Gauge::default()
            .ratio(ratio)
            .label(label);
        frame.render_widget(gauge_section, chunks[1]);

        // Songs section
        let mut songs_datas: Vec<Row> = Vec::new();
        for song in &self.all_songs {
            let (min, sec) = Self::seconds_to_minsec(song.get("duration").unwrap().to_string().parse::<f64>().unwrap());
            let duration = format!("{:02}", min) + ":" + format!("{:02}", sec).as_str();
            songs_datas.push(Row::new(vec![
                song.get("title").unwrap().to_string(),
                song.get("artist").unwrap().to_string(),
                duration,
            ]));
        }
        let header = Row::new(vec!["Title", "Artist", "Duration"]);
        let songs_table = Table::new(
            songs_datas,
            [
                Constraint::Length(80),
                Constraint::Length(30),
                Constraint::Length(10),
            ])
            .header(header)
            .row_highlight_style(Style::default().fg(ratatui::style::Color::Yellow))
            .highlight_symbol(Text::from(vec![" █ ".into()]));
        frame.render_stateful_widget(songs_table, songs, &mut self.state_table);

        // Hotkeys section
        let hotkeys_text = Block::default()
            .title(Line::from("Move up <Up> - Move down <Down> - Play <Enter> - Play/Pause <Space> - Skip <Right> - Quit <Q>").centered());
        frame.render_widget(hotkeys_text, hotkeys);
    }

    // Exit the app on key pressed
    fn exit(&mut self) {
        self.is_running = false;
    }
}
