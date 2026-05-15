#[derive(Debug, thiserror::Error)]
pub enum PaniniError {
    #[error("vidya connection failed: {0}")]
    VidyaConnection(String),
    #[error("failed to parse rule: {0}")]
    RuleParse(String),
    #[error("invalid argument for {tool}.{argument}: {constraint} (got: {received})")]
    InvalidArgument {
        tool: String,
        argument: String,
        constraint: String,
        received: String,
    },
    #[error("no rules loaded for template '{0}'")]
    NoRulesLoaded(String),
}

pub type Result<T> = std::result::Result<T, PaniniError>;
