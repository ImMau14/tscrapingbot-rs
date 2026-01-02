// Prompts Types and Enums

pub struct AiPrompt {
    pub html: String,
    pub thinking: String,
    pub think_and_format: String,
    pub preprocess: String,
}

pub enum Prompt {
    Html,
    Thinking,
    ThinkAndFormat,
    Preprocess,
}

impl AiPrompt {
    pub fn new() -> AiPrompt {
        AiPrompt {
            html: include_str!("./prompts/html.md").to_string(),
            thinking: include_str!("./prompts/thinking.md").to_string(),
            think_and_format: include_str!("./prompts/think_and_format.md").to_string(),
            preprocess: include_str!("./prompts/preprocess.md").to_string(),
        }
    }

    pub fn get(&self, prompt: Prompt) -> String {
        match prompt {
            Prompt::Html => self.html.clone(),
            Prompt::Thinking => self.thinking.clone(),
            Prompt::ThinkAndFormat => self.think_and_format.clone(),
            Prompt::Preprocess => self.preprocess.clone(),
        }
    }
}

impl Default for AiPrompt {
    fn default() -> Self {
        Self::new()
    }
}
