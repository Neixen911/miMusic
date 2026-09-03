use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment as TableAlignment, Constraint, Flex, Layout, Position},
    prelude::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame
};
use std::collections::HashMap;
use std::rc::Rc;

use crate::api::{self, Playlist};
use crate::tools::InputTool;

#[derive(Clone, Debug)]
pub enum Answer {
    BINARY(String, String),
    INPUT(String),
    TABLE(HashMap<String, String>),
    TABLEINPUTS(Vec<(String, String)>)
}

fn get_first(answer: &Answer) -> String {
    match answer {
        Answer::BINARY(..)             => return String::from(""),
        Answer::INPUT(_)               => return String::from(""),
        Answer::TABLE(x)               => return String::from(""),
        Answer::TABLEINPUTS(x) => {
            let first_value = x.first();
            if first_value.is_some() {
                let (name, content) = first_value.expect("Can't first value not be retrieved !");
                return String::from(content)
            } else {
                return String::from("")
            }
        }
    }
}

fn get_constraint(answer: &Answer) -> Constraint {
    match answer {
        &Answer::BINARY(..)             => return Constraint::Max(1),
        &Answer::INPUT(_)               => return Constraint::Max(1),
        &Answer::TABLE(..)              => return Constraint::Max(3),
        &Answer::TABLEINPUTS(_)         => return Constraint::Max(2),
    }
}

// TODO: Make multiple popups instead of only one with all variables inside
#[derive(Clone)]
pub struct PopupTool {
    pub state: TableState,
    pub question: String,
    pub answers: Answer,
    pub is_answer_positive: bool,
    pub input: InputTool
}

impl PopupTool {
    pub fn new(question: String, answers: Answer) -> Self {
        PopupTool {
            state: TableState::default().with_selected(0),
            question: question,
            answers: answers.clone(),
            is_answer_positive: false,
            input: InputTool::new(get_first(&answers))
        }
    }

    pub fn next(&mut self) {
        self.save();
        match &mut self.answers {
            Answer::TABLE(_) => {
                let i = match self.state.selected() {
                    Some(i) => {
                        if i >= api::get_all_playlists().into_iter().len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));
            },
            Answer::TABLEINPUTS(song_infos) => {
                let i = match self.state.selected() {
                    Some(i) => {
                        if i >= song_infos.clone().into_iter().len() - 1 {
                            i
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));
                for (index, (name, content)) in song_infos.clone().iter().enumerate() {
                    if self.state.selected() == Some(index) {
                        self.set_input(content.to_string());
                    }
                }
            },
            
            _ => {}
        }
    }

    pub fn previous(&mut self) {
        match &self.answers {
            Answer::TABLE(song) => {
                let i = match self.state.selected() {
                    Some(i) => {
                        if i == 0 {
                            api::get_all_playlists().into_iter().len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));
            },
            _ => {}
        }
    }

    pub fn save(&mut self) {
        let input = self.get_input_tool().get_input();
        match &mut self.answers {
            Answer::TABLEINPUTS(song_infos) => {
                for (index, (_, content)) in song_infos.iter_mut().enumerate() {
                    if self.state.selected() == Some(index) {
                        *content = input;
                        break;
                    }
                }
            },
            _ => {}
        }
    }

    pub fn get_state(&mut self) -> Option<usize> {
        self.state.selected()
    }

    pub fn switch_answer(&mut self) {
        self.is_answer_positive = !self.is_answer_positive;
    }

    pub fn is_positive(&mut self) -> bool {
        self.is_answer_positive
    }

    pub fn set_answers(&mut self, new_value: Answer) {
        self.answers = new_value;
    }

    pub fn get_answer(&mut self) -> String {
        let mut result = String::from("");
        match &self.answers {
            Answer::TABLE(song) => {
                if self.state.selected().is_some() {
                    let playlists = api::get_all_playlists();
                    let playlist_name = playlists[self.state.selected().expect("Can't retrieve selected id !")].playlist_name.clone();
                    result = String::from(playlist_name);
                }
            },
            _ => { result = String::from(""); }
        }

        result
    }

    pub fn get_answers(&mut self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        match &self.answers {
            Answer::TABLEINPUTS(song_infos) => {
                if self.state.selected().is_some() {
                    result = song_infos.to_vec();
                }
            },
            _ => { result = Vec::new(); }
        }

        result
    }

    pub fn set_input(&mut self, new_value: String) {
        match self.answers {
            Answer::INPUT(_) | Answer::TABLEINPUTS(_) => { self.get_input_tool().set_input(new_value); },
            _ => {}
        }
    }

    pub fn get_input_tool(&mut self) -> &mut InputTool {
        &mut self.input
    }

    pub fn handle_popup_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab                    => { self.switch_answer(); },
            _ => {}
        }
    }

    pub fn render_answers(&mut self, frame: &mut Frame, chunks: Rect, answers: Answer) {
        match answers {
            Answer::BINARY(x, y) => {
                let answers_layout = Layout::horizontal([
                    Constraint::Length(3 + 4),
                    Constraint::Length(2 + 4),
                ])
                .flex(Flex::SpaceBetween)
                .split(chunks);

                let popup_positive_answer = Line::from(x).alignment(Alignment::Center);
                let popup_negative_answer = Line::from(y).alignment(Alignment::Center);
                let popup_positive_answer_style = if self.is_answer_positive {Color::Magenta} else {Color::Reset};
                let popup_negative_answer_style = if !self.is_answer_positive {Color::Magenta} else {Color::Reset};

                frame.render_widget(Paragraph::new(popup_positive_answer)
                    .style(Style::default().bg(popup_positive_answer_style)), answers_layout[0]);
                frame.render_widget(Paragraph::new(popup_negative_answer)
                    .style(Style::default().bg(popup_negative_answer_style)), answers_layout[1]);
            },
            Answer::INPUT(x) => {
                let input_value = self.get_input_tool().get_input();
                let popup_input_answer = Line::from(input_value.as_str()).alignment(Alignment::Center);
                frame.render_widget(Paragraph::new(popup_input_answer)
                    .style(Style::default().bg(Color::Magenta).fg(Color::White)), chunks);
                frame.set_cursor_position(Position::new(
                    chunks.right() - (chunks.right() -  chunks.left()).div_ceil(2) + (self.get_input_tool().get_input().len() as u16).div_ceil(2) - self.get_input_tool().get_position() as u16,
                    chunks.top()
                ));
            },
            Answer::TABLE(song) => {
                let mut formatted_answers = Vec::new();
                let mut playlists_datas: Vec<(String, String)> = Vec::new();
                for playlist in api::get_all_playlists() {
                    let is_in_playlist = playlist.songs_list.contains(song.get("path").expect("Can't retrieve path of song file !"));
                    let checkbox: String;
                    if is_in_playlist || &playlist.playlist_name == "All songs" {
                        checkbox = String::from("[X]");
                    } else { checkbox = String::from("[ ]"); }
                    playlists_datas.push((
                        checkbox,
                        playlist.playlist_name.clone()
                    ));
                }
                for (answer_checkbox, answer_playlist_name) in playlists_datas {
                    formatted_answers.push(Row::new(vec![answer_checkbox, answer_playlist_name]));
                }
                let popup = Table::new(
                    formatted_answers,
                    [
                        Constraint::Length(3),              // Selection box
                        Constraint::Fill(1),                // Playlist name
                    ])
                    .row_highlight_style(Style::default().bg(Color::Magenta).fg(Color::White));
                frame.render_stateful_widget(popup, chunks, &mut self.state);
            },
            Answer::TABLEINPUTS(song_infos) => {
                let mut datas = Vec::new();
                for (entitled_name, entitled_content) in &song_infos {
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
                    datas.push((entitled_name_formatted, entitled_content));
                }
                let order_to_display = vec![
                    "Title".to_string(),
                    "Artist".to_string(),
                ];
                datas.sort_by_key(|(key, _)| {
                    order_to_display
                        .iter()
                        .position(|x| x == key)
                        .unwrap_or(usize::MAX)
                });
                let mut song_datas: Vec<Row> = Vec::new();
                let mut first = true;
                for (index, (name, content)) in datas.iter().enumerate() {
                    if self.state.selected() == Some(index) {
                        let input_value = self.get_input_tool().get_input();
                        song_datas.push(Row::new(vec![
                            Cell::from(format!("{}:", name)), 
                            Cell::from(Line::from(input_value.to_string()).alignment(TableAlignment::Right))
                        ]));
                        frame.set_cursor_position(Position::new(
                            chunks.right() - self.get_input_tool().get_position() as u16,
                            chunks.top() + index as u16
                        ));
                    } else {
                        song_datas.push(Row::new(vec![
                            Cell::from(format!("{}:", name)), 
                            Cell::from(Text::from(content.to_string()).alignment(TableAlignment::Right))
                        ]));
                    }
                }
                let popup = Table::new(
                    song_datas,
                    [
                        Constraint::Length(8),              // Entitled name
                        Constraint::Fill(1),                // Entitled's content
                    ])
                    .row_highlight_style(Style::default().bg(Color::Magenta).fg(Color::White));
                frame.render_stateful_widget(popup, chunks, &mut self.state);
            }
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let popup = frame.area();

        let vertical = Layout::vertical([Constraint::Length(12)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Length(50)]).flex(Flex::Center);
        let [popup] = vertical.areas(popup);
        let [popup] = horizontal.areas(popup);

        let chunks = Layout::vertical([
            Constraint::Max(2),                     // Question's popup
            get_constraint(&self.answers),          // Answers's popup
        ])
        .vertical_margin(3)
        .horizontal_margin(8)
        .flex(Flex::SpaceBetween)
        .split(popup);

        frame.render_widget(Clear, popup);

        let popup_block = Block::bordered();
        frame.render_widget(popup_block, popup);

        let popup_question = Line::from(self.question.clone()).alignment(Alignment::Center);
        frame.render_widget(Paragraph::new(popup_question).wrap(Wrap { trim: true }), chunks[0]);

        self.render_answers(frame, chunks[1], self.answers.clone());
    }
}