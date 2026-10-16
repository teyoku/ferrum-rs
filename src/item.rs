use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(u64);

impl ItemId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

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

    pub fn id(&self) -> ItemId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.name.to_lowercase() == name.trim().to_lowercase()
    }
}
