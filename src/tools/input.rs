#[derive(Clone)]
pub struct InputTool {
    pub input: String,
    pub input_position: usize
}

impl InputTool {
    pub fn new(input: String) -> Self {
        InputTool {
            input: input,
            input_position: 0
        }
    }

    pub fn set_input(&mut self, new_input: String) {
        self.input = new_input;
        self.input_position = 0;
    }

    pub fn get_input(&mut self) -> String {
        self.input.clone()
    }

    pub fn get_position(&mut self) -> usize {
        self.input_position
    }

    pub fn add_char_to_input(&mut self, new_char: char) {
        self.input.insert(self.input.len() - self.input_position, new_char);
    }

    pub fn remove_previous_char_from_input(&mut self) {
        if self.input.len() - self.input_position >= 1 {
            self.input.remove(self.input.len() - self.input_position - 1);
        }
    }

    pub fn remove_next_char_from_input(&mut self) {
        if self.input_position != 0 {
            self.input.remove(self.input.len() - self.input_position);
            self.input_position = self.input_position - 1;
        }
    }

    pub fn left_input_position(&mut self) {
        if self.input_position < self.input.len() {
            self.input_position = self.input_position + 1;
        }
    }

    pub fn right_input_position(&mut self) {
        if self.input_position > 0 {
            self.input_position = self.input_position - 1;
        }
    }
}