use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Gauge, Paragraph},
    Frame
};
use rodio::{OutputStream, OutputStreamBuilder, Sink};

use crate::api;
use crate::ui::{PopupState, Service, ServiceName};

pub struct PlayerService {
    pub service_name: ServiceName,
    pub player: api::Player,
    pub _stream: OutputStream,
}

impl PlayerService {
    // Play or Pause song
    pub fn pause_play_song(&mut self) {
        if !self.player.is_paused() {
            self.player.pause();
        } else { self.player.play(); }
    }

    // Setting loop value
    pub fn next_songs_loop(&mut self) {
        self.player.next_songs_loop();
    }

    // Increment volume
    pub fn increment_volume(&mut self) {
        let volume = self.player.get_volume();
        self.player.set_volume(volume + 0.01);
    }

    // Decrement volume
    pub fn decrement_volume(&mut self) {
        let volume = self.player.get_volume();
        self.player.set_volume(volume - 0.01);
    }

    pub fn set_volume_manual(&mut self, new_volume: f32) {
        self.player.set_volume(new_volume);
    }

    // Add song to queue
    pub fn add_song_to_queue(&mut self, path: String) {
        self.player.add_song_to_queue(path);
    }

    // Skip playing song
    pub fn skip_song(&mut self, skip_direction: u32) {
        if !self.player.empty() {
            self.player.skip_one(skip_direction);
        }
    }

    // Shuffle the queue
    pub fn shuffle_queue(&mut self) {
        self.player.shuffle_queue();
    }

    // Update datas
    pub fn update_datas(&mut self) {
        self.player.update_datas();
    }
}

impl Service for PlayerService {
    fn new(service_name: ServiceName) -> Self {
        let mut stream_handle = OutputStreamBuilder::open_default_stream().expect("Unable to get OutputStreamBuilder !");
        stream_handle.log_on_drop(false);
        PlayerService {
            service_name: service_name,
            player: api::Player::new(Sink::connect_new(&stream_handle.mixer())),
            _stream: stream_handle,
        }
    }

    fn get_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn handle_popup_events(&mut self, _key_event: KeyEvent, _mode: &PopupState) {}

    fn handle_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char(' ')              => { self.pause_play_song(); },
            KeyCode::Up                     => { self.increment_volume(); },
            KeyCode::Down                   => { self.decrement_volume(); },
            KeyCode::Char('t')              => { self.next_songs_loop(); },
            KeyCode::BackTab                => { self.shuffle_queue(); },
            KeyCode::Left                   => { self.skip_song(2); },
            KeyCode::Right                  => { self.skip_song(1); },
            _ => {}
        }
    }

    fn get_hotkeys(&mut self, _mode: &PopupState) -> String {
        String::from("Play/Pause <Space> - Previous <Left> - Skip <Right> - Volume <Up/Down> - Shuffle <Backtab> - Loop <T> - Switch Mode <Tab> - Quit <Q>")
    }

    fn update(&mut self) {
        self.update_datas();
    }

    fn render_popups(&mut self, _frame: &mut Frame, _mode: &PopupState) {}

    fn render(&mut self, frame: &mut Frame, area: Rect, active_service: &ServiceName) {
        let chunks = Layout::vertical([
            Constraint::Length(4),              // Playing informations
            Constraint::Length(1),              // Duration gauge
        ])
        .margin(1)
        .split(area);

        let playing_border_style = if active_service == self.get_name() {Color::Magenta} else {Color::Reset};
        let playing_section = Block::default()
            .title(Line::from("Now Playing"))
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(playing_border_style);
        frame.render_widget(playing_section, area);

        let playing_infos = self.player.get_current_song_info();
        let rows_content: Vec<(String, String)> = vec![
            (
                playing_infos.get(0).expect("Unable to get title from current playing song !").to_string(),
                playing_infos.get(4).expect("Unable to get volume value from player !").to_string()
            ),
            (
                playing_infos.get(1).expect("Unable to get artist from current playing song !").to_string(),
                playing_infos.get(5).expect("Unable to get loop value from player !").to_string()
            )
        ];

        let playing_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); rows_content.len()])
            .split(chunks[0]);

        for (i, (left, right)) in rows_content.iter().enumerate() {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(50),     // Song information
                    Constraint::Min(0),         // Flexible space
                    Constraint::Length(15),     // Player information
                ])
                .split(playing_chunks[i]);

            frame.render_widget(Paragraph::new(Line::from(left.to_string())), cols[0]);
            frame.render_widget(Paragraph::new(Line::from(right.to_string())).right_aligned(), cols[2]);
        }

        let act_duration_song = playing_infos.get(2)
            .expect("Unable to get current duration from current playing song !")
            .to_string()
            .parse::<f64>()
            .expect("Unable to convert into f64 !");
        let max_duration_song = playing_infos.get(3)
            .expect("Unable to get maximum duration from current playing song !")
            .to_string()
            .parse::<f64>()
            .expect("Unable to convert into f64 !");
        let mut ratio = 0.0;
        let (act_minutes, act_seconds) = api::seconds_to_minsec(act_duration_song);
        let (max_minutes, max_seconds) = api::seconds_to_minsec(max_duration_song);
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
    }
}
