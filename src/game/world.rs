use std::collections::HashMap;

use crate::game::room::{Room, RoomId};

pub struct World {
    rooms: HashMap<RoomId, Room>,
}

impl World {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }
}
