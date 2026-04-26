use ratatui::{
    layout::Constraint,
};

pub enum AnswerType {
    BINARY,
    INPUT,
    TABLE,
    TABLEINPUTS
}

fn get_constraint(answer_type: AnswerType) -> Constraint {
    match answer_type {
        AnswerType::BINARY              => return Constraint::Max(1),
        AnswerType::INPUT               => return Constraint::Max(1),
        AnswerType::TABLE               => return Constraint::Max(3),
        AnswerType::TABLEINPUTS         => return Constraint::Max(2),
    }
}

pub struct PopupTool {
    pub question: String,
    pub answers_type: AnswerType,
    pub answers_value: Vec<T>
}

impl PopupTool {
    pub fn new(question: String, answers_type: AnswerType, answers_value: Vec<T>) -> Self {
        PopupTool {
            question: question,
            answers_type: answers_type,
            answers_value: answers_value
        }
    }
}