use crate::game::room::RoomId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(u32);

impl PlayerId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

pub struct Player {
    name: String,
    room: RoomId,
}

impl Player {
    pub fn new(name: impl Into<String>, room: RoomId) -> Self {
        Self {
            name: name.into(),
            room,
        }
    }
}
