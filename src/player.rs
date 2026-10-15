use std::{error::Error, fmt};

use crate::{item::Item, world::RoomId};

pub const MIN_NAME_LENGTH: usize = 3;
pub const MAX_NAME_LENGTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    name: String,
    room: RoomId,
    inventory: Vec<Item>,
}

impl Player {
    pub fn new(name: &str, room: RoomId) -> Result<Self, PlayerNameError> {
        let name = Self::validate_name(name)?;

        Ok(Self {
            name: name.to_owned(),
            room,
            inventory: Vec::new(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn room(&self) -> RoomId {
        self.room
    }

    pub fn inventory(&self) -> &[Item] {
        &self.inventory
    }

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
