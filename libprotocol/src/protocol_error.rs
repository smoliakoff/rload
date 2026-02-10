use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Failed to read scenario file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse scenario JSON: {0}")]
    Json(#[from] JsonError),

    #[error(transparent)]
    Validation(#[from] ValidationErrors),
}

#[derive(Debug, Error)]
#[error("invalid json at line {line}, column {column}: {message}")]
pub struct JsonError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Debug, Error)]
#[error("validation error at [{path}] {message} ({code})")]
pub struct ValidationError {
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug)]
pub struct ValidationErrors {
    pub items: Vec<ValidationError>,
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // коротко, чтобы не спамить
        for err in &self.items {
            writeln!(f, "{}", &err)?
        }
        Ok(())

    }
}

impl std::error::Error for ValidationErrors {}