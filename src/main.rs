#![feature(str_split_remainder)]

use std::io;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use rodio::{OutputStream, Sink};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout},
    prelude::{Alignment},
    style::{Style},
    text::{Line, Text},
    widgets::{Block, Cell, ListItem, Paragraph, Row, Table, TableState},
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
    state: TableState,
    player: music::Player,
    nb_songs: usize,
    is_running: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // Initialisation
        self.is_running = true;
        self.state = TableState::default().with_selected(0);
        self.player = music::Player { m_song_infos: Vec::new(), end_of_song_signal: Arc::new(AtomicU32::new(0)) };
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();

        while self.is_running {
            terminal.draw(|frame| self.draw(frame, &sink))?;
            self.handle_events(&sink)?;
        }
        Ok(())
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self, sink: &Sink) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event, sink)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent, sink: &Sink) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Enter => {
                let song_name = "songs/song0.mp3";
                self.player.m_song_infos.push(music::get_song_infos_from_file(&song_name));
                music::add_song_to_queue(sink, &song_name, &mut self.player);
            },
            KeyCode::Up => self.previous_song(),
            KeyCode::Down => self.next_song(),
            _ => {}
        }
    }

    fn next_song(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.nb_songs - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous_song(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.nb_songs - 1 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn draw(&mut self, frame: &mut Frame, sink: &Sink) {
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
        let playing_info = music::get_current_song_info(sink, &mut self.player);
        let mut playing_lines: Vec<Line> = Vec::new();
        playing_lines.push(Line::from(playing_info.get(0).unwrap().to_string()));
        playing_lines.push(Line::from(playing_info.get(1).unwrap().to_string()));
        playing_lines.push(Line::from(playing_info.get(2).unwrap().to_string()));
        let playing_section = Paragraph::new(playing_lines)
            .block(
                Block::default()
                .title(Line::from("Now Playing"))
                .borders(ratatui::widgets::Borders::ALL)
            );
        frame.render_widget(playing_section, playing);

        // Songs section
        let all_songs = music::get_all_songs();
        self.nb_songs = all_songs.len();
        let mut songs_datas: Vec<Row> = Vec::new();

        for song in all_songs {
            songs_datas.push(Row::new(vec![
                song.get("title").unwrap().to_string(),
                song.get("artist").unwrap().to_string(),
                song.get("duration").unwrap().to_string(),
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
        frame.render_stateful_widget(songs_table, songs, &mut self.state);

        // Hotkeys section
        let hotkeys_text = Block::default()
            .title(Line::from("Move up <↑> - Move down <↓> - Play <Enter> - Quit <Q>").centered());
        frame.render_widget(hotkeys_text, hotkeys);
    }

    fn exit(&mut self) {
        self.is_running = false;
    }
}
