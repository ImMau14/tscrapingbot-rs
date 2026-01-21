// Prompts Types and Enums

pub struct AiPrompt {
    pub html: String,
    pub thinking: String,
    pub think_and_format: String,
    pub preprocess: String,
    pub web_search: String,
    pub vision: String,
}

pub enum Prompt {
    Html,
    Thinking,
    ThinkAndFormat,
    Preprocess,
    WebSearch,
    Vision,
}

impl AiPrompt {
    pub fn new() -> AiPrompt {
        AiPrompt {
            html: include_str!("./prompts/html.md").to_string(),
            thinking: include_str!("./prompts/thinking.md").to_string(),
            think_and_format: include_str!("./prompts/think_and_format.md").to_string(),
            preprocess: include_str!("./prompts/preprocess.md").to_string(),
            web_search: include_str!("./prompts/web_search.md").to_string(),
            vision: include_str!("./prompts/vision.md").to_string(),
        }
    }

    pub fn get(&self, prompt: Prompt) -> String {
        match prompt {
            Prompt::Html => self.html.clone(),
            Prompt::Thinking => self.thinking.clone(),
            Prompt::ThinkAndFormat => self.think_and_format.clone(),
            Prompt::Preprocess => self.preprocess.clone(),
            Prompt::WebSearch => self.web_search.clone(),
            Prompt::Vision => self.vision.clone(),
        }
    }
}

impl Default for AiPrompt {
    fn default() -> Self {
        Self::new()
    }
}
