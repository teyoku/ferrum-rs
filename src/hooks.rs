use std::fmt::Debug;

use crate::world::{Direction, RoomId};

pub struct MoveEvent {
    pub player_name: String,
    pub from: RoomId,
    pub to: RoomId,
    pub direction: Direction,
}

pub trait Hook: Debug + Send + Sync {
    fn before_move(&self, _event: &MoveEvent) -> Result<(), &'static str> {
        Ok(())
    }

    fn after_move(&self, _event: &MoveEvent) {}
}
