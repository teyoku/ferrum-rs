use std::fmt;

/// A stable identifier assigned to an item by a [`crate::World`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(u64);

impl ItemId {
    /// Creates an identifier from its numeric representation.
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An item that can be stored in a room or carried by a player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    id: ItemId,
    name: String,
    description: String,
}

impl Item {
    pub(crate) fn new(id: ItemId, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: description.into(),
        }
    }

    /// Returns the item's stable identifier.
    pub fn id(&self) -> ItemId {
        self.id
    }

    /// Returns the name shown to players and used for lookups.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the description shown by the `examine` command.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns whether `name` identifies this item.
    ///
    /// Matching ignores surrounding whitespace and Unicode case differences.
    pub fn matches_name(&self, name: &str) -> bool {
        self.name.to_lowercase() == name.trim().to_lowercase()
    }
}
