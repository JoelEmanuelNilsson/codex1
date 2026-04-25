use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Argument,
    MissionPath,
    ArtifactValidation,
    Io,
    Template,
    Interview,
    Loop,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Argument => "ARGUMENT_ERROR",
            Self::MissionPath => "MISSION_PATH_ERROR",
            Self::ArtifactValidation => "ARTIFACT_VALIDATION_ERROR",
            Self::Io => "IO_ERROR",
            Self::Template => "TEMPLATE_ERROR",
            Self::Interview => "INTERVIEW_ERROR",
            Self::Loop => "LOOP_ERROR",
        }
    }
}

#[derive(Debug, Error)]
pub enum Codex1Error {
    #[error("{0}")]
    Argument(String),
    #[error("{0}")]
    MissionPath(String),
    #[error("{0}")]
    ArtifactValidation(String),
    #[error("{0}")]
    Template(String),
    #[error("{0}")]
    Interview(String),
    #[error("{0}")]
    Loop(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl Codex1Error {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Argument(_) => ErrorCode::Argument,
            Self::MissionPath(_) => ErrorCode::MissionPath,
            Self::ArtifactValidation(_) => ErrorCode::ArtifactValidation,
            Self::Template(_) => ErrorCode::Template,
            Self::Interview(_) | Self::Json { .. } => ErrorCode::Interview,
            Self::Loop(_) => ErrorCode::Loop,
            Self::Io { .. } => ErrorCode::Io,
        }
    }
}

pub type Result<T> = std::result::Result<T, Codex1Error>;

pub trait IoContext<T> {
    fn io_context(self, context: impl Into<String>) -> Result<T>;
}

impl<T> IoContext<T> for std::io::Result<T> {
    fn io_context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|source| Codex1Error::Io {
            context: context.into(),
            source,
        })
    }
}
