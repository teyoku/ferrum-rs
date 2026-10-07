use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RoomId(u32);

impl RoomId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

pub struct Room {
    title: String,
    description: String,
    exits: HashMap<String, RoomId>,
}

impl Room {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            exits: HashMap::new(),
        }
    }
}
