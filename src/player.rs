use std::{error::Error, fmt};

pub const MIN_NAME_LENGTH: usize = 3;
pub const MAX_NAME_LENGTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    name: String,
}

impl Player {
    pub fn new(name: &str) -> Result<Self, PlayerNameError> {
        let name = name.trim();
        let length = name.chars().count();

        if length < MIN_NAME_LENGTH {
            return Err(PlayerNameError::TooShort);
        }

        if length > MAX_NAME_LENGTH {
            return Err(PlayerNameError::TooLong);
        }

        if !name.chars().all(|c| c.is_alphabetic()) {
            return Err(PlayerNameError::InvalidCharacters);
        }

        Ok(Self {
            name: name.to_owned(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerNameError {
    TooShort,
    TooLong,
    InvalidCharacters,
    AlreadyInUse,
    SessionNotFound,
}

impl fmt::Display for PlayerNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            PlayerNameError::TooShort => "name must contain at least 3 letters",
            PlayerNameError::TooLong => "name must contain at most 16 letters",
            PlayerNameError::InvalidCharacters => "name may contain letters only",
            PlayerNameError::AlreadyInUse => "that name is already in use",
            PlayerNameError::SessionNotFound => "session is no longer connected",
        };

        f.write_str(message)
    }
}

impl Error for PlayerNameError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_valid_name() {
        let player = Player::new("Alice").unwrap();

        assert_eq!(player.name(), "Alice");
    }

    #[test]
    fn rejects_a_name_with_non_letters() {
        assert_eq!(
            Player::new("Alice42"),
            Err(PlayerNameError::InvalidCharacters)
        )
    }
}
