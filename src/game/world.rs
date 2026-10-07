use std::collections::HashMap;

use crossbeam_channel::Receiver;

use crate::game::{
    event::{self, ClientId, GameEvent}, player::{Player, PlayerId}, room::{Room, RoomId},
};

pub struct World {
    rooms: HashMap<RoomId, Room>,
    players: HashMap<PlayerId, Player>,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            rooms: HashMap::new(),
            players: HashMap::new(),
        };
        world
            .rooms
            .insert(RoomId::new(1), Room::new("start_room", "Just a start room"));

        world
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

        let player = Player::new(player_name, RoomId::new(1));

        self.players.insert(player_id, player);
    }
}

pub fn run_game_world(event_rx: Receiver<GameEvent>) {
    let mut world = World::new();

    while let Ok(event) = event_rx.recv() {
        world.handle(event);
    }
}