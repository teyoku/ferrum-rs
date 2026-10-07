#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(u32);

pub enum GameEvent {
    Connected {
        id: ClientId
    },

    Command {
        id: ClientId,
        text: String
    },

    Disconnected {
        id: ClientId
    },

    Shutdown
}