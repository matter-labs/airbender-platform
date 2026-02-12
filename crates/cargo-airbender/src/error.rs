use anyhow::Error as AnyhowError;

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct CliError {
    message: String,
    #[source]
    source: Option<AnyhowError>,
    hint: Option<String>,
}

impl CliError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
            hint: None,
        }
    }

    pub fn with_source(message: impl Into<String>, source: impl Into<AnyhowError>) -> Self {
        Self {
            message: message.into(),
            source: Some(source.into()),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn hint(&self) -> Option<&str> {
        self.hint.as_deref()
    }

    pub fn source_error(&self) -> Option<&AnyhowError> {
        self.source.as_ref()
    }
}

pub type Result<T> = std::result::Result<T, CliError>;
