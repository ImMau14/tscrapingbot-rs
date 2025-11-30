// Prompts Types and Enums

pub struct GeminiPrompt {
    pub html: String,
    pub thinking: String,
    pub think_and_format: String,
}

pub enum Prompt {
    Html,
    Thinking,
    ThinkAndFormat,
}

impl GeminiPrompt {
    pub fn new() -> GeminiPrompt {
        GeminiPrompt {
            html: include_str!("./prompts/html.md").to_string(),
            thinking: include_str!("./prompts/gemini_thinking.md").to_string(),
            think_and_format: include_str!("./prompts/think_and_format.md").to_string(),
        }
    }

    pub fn get(&self, prompt: Prompt) -> String {
        match prompt {
            Prompt::Html => self.html.clone(),
            Prompt::Thinking => self.thinking.clone(),
            Prompt::ThinkAndFormat => self.think_and_format.clone(),
        }
    }
}

impl Default for GeminiPrompt {
    fn default() -> Self {
        Self::new()
    }
}
