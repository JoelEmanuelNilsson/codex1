use std::io;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("missing YAML frontmatter")]
    MissingFrontmatter,
    #[error("invalid YAML frontmatter delimiter")]
    InvalidFrontmatterDelimiter,
    #[error("artifact kind mismatch: expected `{expected}`, found `{found}`")]
    ArtifactKindMismatch {
        expected: &'static str,
        found: String,
    },
    #[error("invalid fingerprint `{0}`")]
    InvalidFingerprint(String),
    #[error("invalid reason code `{0}`")]
    InvalidReasonCode(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
}
