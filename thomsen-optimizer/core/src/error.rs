//! One structured error type for the whole core. It serialises to
//! `{ "code": "...", "message": "..." }` so the UI can show a plain message
//! and still branch on the code (e.g. `permission_denied` -> offer elevation).

use serde::{ser::SerializeStruct, Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    PermissionDenied(String),
    #[error("{0}")]
    Command(String),
    #[error("{0}")]
    Busy(String),
    #[error("storage error: {0}")]
    Io(#[from] std::io::Error),
    #[error("data error: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Error::Invalid(_) => "invalid",
            Error::NotFound(_) => "not_found",
            Error::Unsupported(_) => "unsupported",
            Error::PermissionDenied(_) => "permission_denied",
            Error::Command(_) => "command_failed",
            Error::Busy(_) => "busy",
            Error::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => "permission_denied",
            Error::Io(_) => "io",
            Error::Json(_) => "data",
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Error", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}
