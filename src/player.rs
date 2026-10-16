use std::{error::Error, fmt};

use crate::{item::Item, world::RoomId};

/// Minimum number of Unicode characters permitted in a player name.
pub const MIN_NAME_LENGTH: usize = 3;
/// Maximum number of Unicode characters permitted in a player name.
pub const MAX_NAME_LENGTH: usize = 16;

/// A named player, their current room, and carried items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    name: String,
    room: RoomId,
    inventory: Vec<Item>,
}

impl Player {
    /// Creates a player in `room` after validating `name`.
    ///
    /// Names are trimmed, must contain between [`MIN_NAME_LENGTH`] and
    /// [`MAX_NAME_LENGTH`] Unicode characters, and may contain alphabetic
    /// characters only. Uniqueness is enforced by the server, not this type.
    pub fn new(name: &str, room: RoomId) -> Result<Self, PlayerNameError> {
        let name = Self::validate_name(name)?;

        Ok(Self {
            name: name.to_owned(),
            room,
            inventory: Vec::new(),
        })
    }

    /// Returns the player's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the room currently occupied by the player.
    pub fn room(&self) -> RoomId {
        self.room
    }

    /// Returns all items carried by the player.
    pub fn inventory(&self) -> &[Item] {
        &self.inventory
    }

    /// Consumes the player and returns all carried items.
    pub fn into_inventory(self) -> Vec<Item> {
        self.inventory
    }

    pub(crate) fn rename(&mut self, name: &str) -> Result<(), PlayerNameError> {
        self.name = Self::validate_name(name)?.to_owned();

        Ok(())
    }

    pub(crate) fn move_to(&mut self, room: RoomId) {
        self.room = room;
    }

    pub(crate) fn give_item(&mut self, item: Item) {
        self.inventory.push(item);
    }

    pub(crate) fn take_item(&mut self, name: &str) -> Option<Item> {
        let index = self
            .inventory
            .iter()
            .position(|item| item.matches_name(name))?;

        Some(self.inventory.remove(index))
    }

    /// Finds a carried item by its case-insensitive name.
    pub fn find_item(&self, name: &str) -> Option<&Item> {
        self.inventory.iter().find(|item| item.matches_name(name))
    }

    fn validate_name(name: &str) -> Result<&str, PlayerNameError> {
        let name = name.trim();
        let length = name.chars().count();

        if length < MIN_NAME_LENGTH {
            return Err(PlayerNameError::TooShort);
        }

        if length > MAX_NAME_LENGTH {
            return Err(PlayerNameError::TooLong);
        }

        if !name.chars().all(|character| character.is_alphabetic()) {
            return Err(PlayerNameError::InvalidCharacters);
        }

        Ok(name)
    }
}

/// A reason why a player name could not be accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerNameError {
    /// The name contains fewer than [`MIN_NAME_LENGTH`] characters.
    TooShort,
    /// The name contains more than [`MAX_NAME_LENGTH`] characters.
    TooLong,
    /// The name contains a non-alphabetic character.
    InvalidCharacters,
    /// Another connected player already uses the name.
    AlreadyInUse,
    /// The session disappeared before the name could be assigned.
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
        let player = Player::new("Alice", RoomId::new(1)).unwrap();

        assert_eq!(player.name(), "Alice");
        assert_eq!(player.room(), RoomId::new(1));
    }

    #[test]
    fn rejects_a_name_with_non_letters() {
        assert_eq!(
            Player::new("Alice42", RoomId::new(1)),
            Err(PlayerNameError::InvalidCharacters)
        )
    }
}
