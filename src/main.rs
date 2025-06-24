#![feature(str_split_remainder)]

use std::io;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout},
    prelude::{Alignment},
    style::{Style},
    text::{Line, Text},
    widgets::{Block, Cell, ListItem, Paragraph, Row, Table},
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
    player: music::Player,
    is_running: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.is_running = true;
        self.player = music::Player { m_song_infos: Vec::new(), end_of_song_signal: Arc::new(AtomicU32::new(0)) };
        while self.is_running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Fill(100),
            Constraint::Min(2),
        ]);
        let [app, playing, songs, hotkeys] = vertical.areas(frame.area());

        // Application section
        let app_text = Block::default()
            .title(Line::from(" miMusic ").centered());
        frame.render_widget(app_text, app);

        // Playing section
        // todo!();

        // Songs section
        let all_songs = music::get_all_songs();
        let mut songs_datas: Vec<Row> = Vec::new();

        for song in all_songs {
            songs_datas.push(Row::new(vec![
                song.get("title").unwrap().to_string(),
                song.get("artist").unwrap().to_string(),
                song.get("duration").unwrap().to_string(),
            ]));
        }
        let header = Row::new(vec!["Song", "Artist", "Duration"]);
        let songs_table = Table::new(
            songs_datas,
            [
                Constraint::Length(65 + 1),
                Constraint::Length(30 + 1),
                Constraint::Length(8),
            ])
            .header(header);
        frame.render_widget(songs_table, songs);

        // Hotkeys section
        let hotkeys_text = Block::default()
            .title(Line::from("Quit <Q>").centered());
        frame.render_widget(hotkeys_text, hotkeys);
    }

    fn exit(&mut self) {
        self.is_running = false;
    }
}
