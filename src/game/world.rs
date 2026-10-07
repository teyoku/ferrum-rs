use std::collections::HashMap;

use crate::game::{
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
}
