#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(u32);

impl ClientId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

pub enum GameEvent {
    Connected { id: ClientId },

    Command { id: ClientId, text: String },

    Disconnected { id: ClientId },

    Shutdown,
}
