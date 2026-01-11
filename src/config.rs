use dotenvy::dotenv;
use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("missing environment variable: {0}")]
    MissingEnv(&'static str),
    #[error("invalid HOSTING value (expected true|false): {0}")]
    InvalidHosting(String),
    #[error("invalid WEBHOOK_URL: {0}")]
    InvalidWebhookUrl(String),
}

#[derive(Clone)]
pub struct Models {
    pub vision: String,
    pub preprocessing: String,
    pub thinking: String,
}

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub scrapedo_token: String,
    pub token: String,
    pub groq_api_key: String,
    pub hosting: bool,
    pub webhook_url: Option<url::Url>,
    pub port: u16,
    pub models: Models,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("database_url", &"<redacted>")
            .field("token", &"<redacted>")
            .field("scrapedo_token", &"<redacted>")
            .field("groq_api_key", &"<redacted>")
            .field("hosting", &self.hosting)
            .field("webhook_url", &self.webhook_url)
            .field("port", &self.port)
            .finish()
    }
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let load_dotenv = match env::var("DOTENV_DISABLE") {
            Ok(val) => {
                let low = val.to_lowercase();
                !(low == "1" || low == "true" || low == "yes")
            }
            Err(_) => true,
        };

        if load_dotenv {
            let _ = dotenv();
        }

        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::MissingEnv("DATABASE_URL"))?;

        let token =
            env::var("TELOXIDE_TOKEN").map_err(|_| ConfigError::MissingEnv("TELOXIDE_TOKEN"))?;

        let scrapedo_token =
            env::var("SCRAPEDO_TOKEN").map_err(|_| ConfigError::MissingEnv("SCRAPEDO_TOKEN"))?;

        let groq_api_key =
            env::var("GROQ_API_KEY").map_err(|_| ConfigError::MissingEnv("GROQ_API_KEY"))?;

        let hosting_raw = env::var("HOSTING").map_err(|_| ConfigError::MissingEnv("HOSTING"))?;

        let hosting = match hosting_raw.to_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" => false,
            other => return Err(ConfigError::InvalidHosting(other.to_string())),
        };

        let webhook_url = match env::var("WEBHOOK_URL") {
            Ok(s) if !s.trim().is_empty() => {
                let parsed =
                    url::Url::parse(&s).map_err(|_| ConfigError::InvalidWebhookUrl(s.clone()))?;
                Some(parsed)
            }
            _ => None,
        };

        let port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8080);

        // Fix: read model env vars with defaults
        let vision = env::var("VISION_MODEL")
            .unwrap_or_else(|_| "meta-llama/llama-4-scout-17b-16e-instruct".to_string());
        let preprocessing =
            env::var("PREPROCESSING_MODEL").unwrap_or_else(|_| "openai/gpt-oss-20b".to_string());
        let thinking =
            env::var("THINKING_MODEL").unwrap_or_else(|_| "openai/gpt-oss-120b".to_string());

        Ok(Self {
            database_url,
            token,
            scrapedo_token,
            groq_api_key,
            hosting,
            webhook_url,
            port,
            models: Models {
                vision,
                preprocessing,
                thinking,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn from_env_parses_all() {
        unsafe {
            env::set_var("DOTENV_DISABLE", "1");
        }

        unsafe {
            env::set_var("DATABASE_URL", "postgresql://hello");
            env::set_var("TELOXIDE_TOKEN", "tok");
            env::set_var("SCRAPEDO_TOKEN", "scrape123");
            env::set_var("GROQ_API_KEY", "asdfg");
            env::set_var("HOSTING", "true");
            env::set_var("WEBHOOK_URL", "https://example.com/hook");
            env::set_var("PORT", "1234");
            // optional model vars not set, defaults will be used
        }

        let cfg = AppConfig::from_env().unwrap();
        assert_eq!(cfg.database_url, "postgresql://hello");
        assert_eq!(cfg.token, "tok");
        assert_eq!(cfg.scrapedo_token, "scrape123");
        assert_eq!(cfg.groq_api_key, "asdfg");
        assert!(cfg.hosting);
        assert_eq!(cfg.port, 1234);
        assert_eq!(
            cfg.webhook_url.unwrap().as_str(),
            "https://example.com/hook"
        );
        // Verify defaults
        assert_eq!(
            cfg.models.vision,
            "meta-llama/llama-4-scout-17b-16e-instruct"
        );
        assert_eq!(cfg.models.preprocessing, "openai/gpt-oss-20b");
        assert_eq!(cfg.models.thinking, "openai/gpt-oss-120b");

        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("TELOXIDE_TOKEN");
            env::remove_var("SCRAPEDO_TOKEN");
            env::remove_var("GROQ_API_KEY");
            env::remove_var("HOSTING");
            env::remove_var("WEBHOOK_URL");
            env::remove_var("PORT");
        }

        unsafe {
            env::remove_var("DOTENV_DISABLE");
        }
    }

    #[test]
    #[serial]
    fn from_env_missing_token() {
        unsafe {
            env::set_var("DOTENV_DISABLE", "1");
        }

        unsafe {
            env::set_var("DATABASE_URL", "postgresql://dummy");
            env::remove_var("TELOXIDE_TOKEN");
            env::set_var("SCRAPEDO_TOKEN", "scrape123");
            env::set_var("GROQ_API_KEY", "HELLO");
            env::set_var("HOSTING", "false");
        }

        let res = AppConfig::from_env();
        match res {
            Err(ConfigError::MissingEnv("TELOXIDE_TOKEN")) => {}
            other => panic!("expected MissingEnv TELOXIDE_TOKEN, got {:?}", other),
        }

        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("GROQ_API_KEY");
            env::remove_var("HOSTING");
            env::remove_var("SCRAPEDO_TOKEN");
            env::remove_var("DOTENV_DISABLE");
        }
    }
}
