use std::{borrow::Cow, error::Error, fmt};

/// An error produced while executing game logic.
///
/// The message is suitable for display to a connected player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameError {
    message: Cow<'static, str>,
}

impl GameError {
    /// Creates an error from a static or owned message.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the player-facing error message.
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

/// The standard result type used by commands and hooks.
pub type GameResult<T = ()> = Result<T, GameError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_owned_contextual_messages() {
        let error = GameError::new(format!("the {} gate is closed", "north"));

        assert_eq!(error.message(), "the north gate is closed");
        assert_eq!(error.to_string(), "the north gate is closed");
    }
}
