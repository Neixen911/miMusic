use ratatui::{
    layout::{Constraint, Flex, Layout},
    prelude::{Alignment},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame
};

#[derive(Clone)]
pub enum Answer {
    BINARY(String, String),
    INPUT(String),
    TABLE(Vec<(String, String)>),
    TABLEINPUTS(Vec<(String, String)>)
}

impl IntoIterator for Answer {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<(String, String)>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Answer::TABLE(x)            => x.into_iter(),
            Answer::TABLEINPUTS(x)      => x.into_iter(),
            _                           => vec![].into_iter(),
        }
    }
}

fn get_constraint(answer: &Answer) -> Constraint {
    match answer {
        &Answer::BINARY(..)             => return Constraint::Max(1),
        &Answer::INPUT(_)               => return Constraint::Max(1),
        &Answer::TABLE(_)               => return Constraint::Max(3),
        &Answer::TABLEINPUTS(_)         => return Constraint::Max(2),
    }
}

pub struct PopupTool {
    pub state: TableState,
    pub question: String,
    pub answers: Answer,
}

impl PopupTool {
    pub fn new(question: String, answers: Answer) -> Self {
        PopupTool {
            state: TableState::default().with_selected(0),
            question: question,
            answers: answers
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

        let mut answers = Vec::new();
        for (answer_checkbox, answer_playlist_name) in self.answers.clone() {
            answers.push(Row::new(vec![answer_checkbox, answer_playlist_name]));
        }
        let popup_answers = Table::new(
            answers,
            [
                Constraint::Length(3),              // Selection box
                Constraint::Fill(1),                // Playlist name
            ])
            .row_highlight_style(Style::default().bg(Color::Magenta).fg(Color::White));
        frame.render_stateful_widget(popup_answers, chunks[1], &mut self.state);
    }
}