use std::collections::HashMap;

use crossbeam_channel::Receiver;

use crate::game::{
    event::{ClientId, GameEvent},
    player::{Player, PlayerId},
    room::{Room, RoomId},
};

pub struct World {
    rooms: HashMap<RoomId, Room>,
    players: HashMap<PlayerId, Player>,
    connections: HashMap<ClientId, PlayerId>,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            rooms: HashMap::new(),
            players: HashMap::new(),
            connections: HashMap::new(),
        };
        world
            .rooms
            .insert(RoomId::new(1), Room::new("start_room", "Just a start room"));

        world
    }

    pub fn handle(&mut self, event: GameEvent) {
        match event {
            GameEvent::Connected { id } => self.connect(id),
            GameEvent::Command { id, text } => self.command(id, text),
            GameEvent::Disconnected { id } => self.disconnect(id),
            GameEvent::Shutdown => todo!(),
        }
    }

    pub fn connect(&mut self, id: ClientId) {
        let player_id = PlayerId::new(id.value());
        let player_name = format!("Player_{}", player_id.value());

        let player = Player::new(player_name, RoomId::new(1));

        self.players.insert(player_id, player);
        self.connections.insert(id, player_id);

        println!("Player_{} has connected.", player_id.value());
    }

    pub fn command(&mut self, id: ClientId, text: String) {
        let Some(&player_id) = self.connections.get(&id) else {
            println!("Command from unknown client.");
            return;
        };

        println!("Player_{}'s input: {text}", player_id.value());
    }

    pub fn disconnect(&mut self, id: ClientId) {
        let Some(player_id) = self.connections.remove(&id) else {
            println!("Disconnect from unknown client.");
            return;
        };

        self.players.remove(&player_id);
        println!("Player_{} has disconnected", player_id.value());
    }
}

pub fn run_game_world(event_rx: Receiver<GameEvent>) {
    let mut world = World::new();

    while let Ok(event) = event_rx.recv() {
        world.handle(event);
    }
}
