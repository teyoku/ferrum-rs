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
    Room {
        room_id: RoomId,
        except: Option<ClientId>,
    },
    All,
}

pub enum EngineEffect {
    Send {
        session_id: ClientId,
        message: String,
    },
    Broadcast {
        audience: Audience,
        message: String,
    },
    Close {
        session_id: ClientId,
        reason: String,
    },
}

pub enum GameEvent {
    Connected { id: ClientId },

    Command { id: ClientId, text: String },

    Disconnected { id: ClientId },
}
