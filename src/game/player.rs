use crossbeam_channel::Sender;

use crate::game::room::RoomId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(u32);

impl PlayerId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

pub struct Player {
    name: String,
    room: RoomId,
    tx: Sender<String>,
}

impl Player {
    pub fn new(name: impl Into<String>, room: RoomId, tx: Sender<String>) -> Self {
        Self {
            name: name.into(),
            room,
            tx,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn room(&self) -> RoomId {
        self.room
    }

    pub fn tx(&self) -> &Sender<String> {
        &self.tx
    }

    pub fn move_to(&mut self, room: RoomId) {
        self.room = room;
    }
}
