use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub vidya_url: String,
    pub vidya_auth_token: Option<String>,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            vidya_url: env_or("VIDYA_URL", "http://127.0.0.1:3300/mcp"),
            vidya_auth_token: env::var("VIDYA_AUTH_TOKEN").ok(),
            log_level: env_or("PANINI_LOG_LEVEL", "info"),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}
