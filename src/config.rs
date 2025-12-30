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
pub struct AppConfig {
    pub database_url: String,
    pub token: String,
    pub gemini_api_key: String,
    pub hosting: bool,
    pub webhook_url: Option<url::Url>,
    pub port: u16,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("database_url", &"<redacted>")
            .field("token", &"<redacted>")
            .field("gemini_api_key", &"<redacted>")
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
            let _ = dotenv(); // idempotente y no sobreescribe variables existentes
        }

        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::MissingEnv("DATABASE_URL"))?;

        let token =
            env::var("TELOXIDE_TOKEN").map_err(|_| ConfigError::MissingEnv("TELOXIDE_TOKEN"))?;

        let gemini_api_key =
            env::var("GEMINI_API_KEY").map_err(|_| ConfigError::MissingEnv("GEMINI_API_KEY"))?;

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

        Ok(Self {
            database_url,
            token,
            gemini_api_key,
            hosting,
            webhook_url,
            port,
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
            env::set_var("GEMINI_API_KEY", "asd");
            env::set_var("HOSTING", "true");
            env::set_var("WEBHOOK_URL", "https://example.com/hook");
            env::set_var("PORT", "1234");
        }

        let cfg = AppConfig::from_env().unwrap();
        assert_eq!(cfg.database_url, "postgresql://hello");
        assert_eq!(cfg.token, "tok");
        assert_eq!(cfg.gemini_api_key, "asd");
        assert!(cfg.hosting);
        assert_eq!(cfg.port, 1234);
        assert_eq!(
            cfg.webhook_url.unwrap().as_str(),
            "https://example.com/hook"
        );

        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("TELOXIDE_TOKEN");
            env::remove_var("GEMINI_API_KEY");
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
            env::remove_var("TELOXIDE_TOKEN"); // lo que probamos
            env::set_var("GEMINI_API_KEY", "dummykey");
            env::set_var("HOSTING", "false");
        }

        let res = AppConfig::from_env();
        match res {
            Err(ConfigError::MissingEnv("TELOXIDE_TOKEN")) => {}
            other => panic!("expected MissingEnv TELOXIDE_TOKEN, got {:?}", other),
        }

        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("GEMINI_API_KEY");
            env::remove_var("HOSTING");
            env::remove_var("DOTENV_DISABLE");
        }
    }
}
