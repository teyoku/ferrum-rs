use std::collections::HashMap;

use crate::game::{
    event::GameEvent,
    player::{Player, PlayerId},
    room::{Room, RoomId},
};

pub struct World {
    rooms: HashMap<RoomId, Room>,
    players: HashMap<PlayerId, Player>,
}

impl World {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            players: HashMap::new(),
        }
    }

    pub fn handle(&self, event: GameEvent) {
        match event {
            GameEvent::Connected { id } => todo!(),
            GameEvent::Command { id, text } => todo!(),
            GameEvent::Disconnected { id } => todo!(),
            GameEvent::Shutdown => todo!(),
        }
    }
}
