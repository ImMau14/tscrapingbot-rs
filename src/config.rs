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

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub token: String,
    pub hosting: bool,
    pub webhook_url: Option<url::Url>,
    pub port: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        if cfg!(not(test)) {
            let _ = dotenv();
        }

        let token =
            env::var("TELOXIDE_TOKEN").map_err(|_| ConfigError::MissingEnv("TELOXIDE_TOKEN"))?;

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
            .unwrap_or(8080u16);

        Ok(AppConfig {
            token,
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
            env::set_var("TELOXIDE_TOKEN", "tok");
            env::set_var("HOSTING", "true");
            env::set_var("WEBHOOK_URL", "https://example.com/hook");
            env::set_var("PORT", "1234");
        }

        let cfg = AppConfig::from_env().unwrap();
        assert_eq!(cfg.token, "tok");
        assert!(cfg.hosting);
        assert_eq!(cfg.port, 1234);
        assert_eq!(
            cfg.webhook_url.unwrap().as_str(),
            "https://example.com/hook"
        );

        unsafe {
            env::remove_var("TELOXIDE_TOKEN");
            env::remove_var("HOSTING");
            env::remove_var("WEBHOOK_URL");
            env::remove_var("PORT");
        }
    }

    #[test]
    #[serial]
    fn from_env_missing_token() {
        unsafe {
            env::remove_var("TELOXIDE_TOKEN");
            env::set_var("HOSTING", "false");
        }

        let res = AppConfig::from_env();
        match res {
            Err(ConfigError::MissingEnv("TELOXIDE_TOKEN")) => {}
            other => panic!("expected MissingEnv TELOXIDE_TOKEN, got {:?}", other),
        }

        unsafe {
            env::remove_var("HOSTING");
        }
    }
}
