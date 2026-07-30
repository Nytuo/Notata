use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotataError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Lofty error: {0}")]
    Lofty(#[from] lofty::error::LoftyError),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Provider error ({provider}): {message}")]
    Provider { provider: String, message: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Custom(String),
}

impl From<NotataError> for String {
    fn from(err: NotataError) -> String {
        err.to_string()
    }
}

pub type Result<T> = std::result::Result<T, NotataError>;
