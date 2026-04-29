use ratatui::{
    layout::Constraint,
    widgets::{Cell, Row},
};

pub enum Answer {
    BINARY(bool),
    INPUT(String),
    TABLE(Vec<(String, String)>),
    TABLEINPUTS(Vec<(String, String)>)
}

fn get_constraint(answer: Answer) -> Constraint {
    match answer {
        Answer::BINARY(_)               => return Constraint::Max(1),
        Answer::INPUT(_)                => return Constraint::Max(1),
        Answer::TABLE(_)                => return Constraint::Max(3),
        Answer::TABLEINPUTS(_)          => return Constraint::Max(2),
    }
}

pub struct PopupTool {
    pub question: String,
    pub answers: Answer,
}

impl PopupTool {
    pub fn new(question: String, answers: Answer) -> Self {
        PopupTool {
            question: question,
            answers: answers
        }
    }
}