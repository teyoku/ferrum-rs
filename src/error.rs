use std::{borrow::Cow, error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameError {
    message: Cow<'static, str>,
}

impl GameError {
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for GameError {}

impl From<&'static str> for GameError {
    fn from(message: &'static str) -> Self {
        Self::new(message)
    }
}

impl From<String> for GameError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

pub type GameResult<T = ()> = Result<T, GameError>;
