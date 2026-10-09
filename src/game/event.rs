use tokio::sync::mpsc::Sender;

use crate::game::room::RoomId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(u32);

impl ClientId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

pub enum Audience {
    Room(RoomId),
    All,
}

pub enum GameEvent {
    Connected { id: ClientId, tx: Sender<String> },

    Command { id: ClientId, text: String },

    Disconnected { id: ClientId },
}
