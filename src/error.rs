use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Argument,
    MissionPath,
    Io,
    SetupBackup,
    SetupRestore,
    SetupBundle,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Argument => "ARGUMENT_ERROR",
            Self::MissionPath => "MISSION_PATH_ERROR",
            Self::Io => "IO_ERROR",
            Self::SetupBackup => "SETUP_BACKUP_ERROR",
            Self::SetupRestore => "SETUP_RESTORE_ERROR",
            Self::SetupBundle => "SETUP_BUNDLE_ERROR",
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
    SetupBackup(String),
    #[error("{0}")]
    SetupRestore(String),
    #[error("{0}")]
    SetupBundle(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
}

impl Codex1Error {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Argument(_) => ErrorCode::Argument,
            Self::MissionPath(_) => ErrorCode::MissionPath,
            Self::SetupBackup(_) => ErrorCode::SetupBackup,
            Self::SetupRestore(_) => ErrorCode::SetupRestore,
            Self::SetupBundle(_) => ErrorCode::SetupBundle,
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
