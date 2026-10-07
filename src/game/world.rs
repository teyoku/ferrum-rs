use std::collections::HashMap;

use crate::game::{
    event::{ClientId, GameEvent}, player::{Player, PlayerId}, room::{Room, RoomId},
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

    pub fn handle(&mut self, event: GameEvent) {
        match event {
            GameEvent::Connected { id } => self.connect(id),
            GameEvent::Command { id, text } => todo!(),
            GameEvent::Disconnected { id } => todo!(),
            GameEvent::Shutdown => todo!(),
        }
    }

    pub fn connect(&mut self, id: ClientId) {
        let player_id = PlayerId::new(id.value());
        let player_name = format!("Player_{}", player_id.value());

        let player = Player::new(player_name, room);

        self.players.insert(player_id, player);
    }
}
