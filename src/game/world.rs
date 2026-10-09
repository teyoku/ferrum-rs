use std::collections::HashMap;

use tokio::sync::mpsc::Sender;

use crate::game::{
    event::ClientId,
    player::{Player, PlayerId},
    room::{Direction, Room, RoomId},
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

    pub fn players(&self) -> &HashMap<PlayerId, Player> {
        &self.players
    }

    pub fn player_id(&self, client_id: ClientId) -> Option<PlayerId> {
        self.connections.get(&client_id).copied()
    }

    pub fn send_to(&self, player_id: PlayerId, message: impl Into<String>) -> Result<(), String> {
        let player = self
            .players
            .get(&player_id)
            .ok_or_else(|| "Player does not exist".to_string())?;

        player
            .tx()
            .send(message.into())
            .map_err(|_| "Client disconnected".to_string())
    }

    pub fn describe_current_room(&self, player_id: PlayerId) -> Result<String, String> {
        let player = self
            .players
            .get(&player_id)
            .ok_or_else(|| "Player does not exist".to_string())?;

        let room = self
            .rooms
            .get(&player.room())
            .ok_or_else(|| "Player room does not exist".to_string())?;

        let mut exits = room
            .exits()
            .keys()
            .map(|direction| direction.as_str())
            .collect::<Vec<_>>();

        exits.sort_unstable();

        Ok(format!(
            "{}\n{}\n\n[Exits: {}]\n",
            room.title(),
            room.description(),
            exits.join(", ")
        ))
    }

    pub fn move_player(&mut self, player_id: PlayerId, direction: Direction) -> Result<(), String> {
        let current_room = self
            .players
            .get(&player_id)
            .ok_or_else(|| "Player does not exist".to_string())?
            .room();

        let destination = self
            .rooms
            .get(&current_room)
            .ok_or_else(|| "Player room does not exist".to_string())?
            .exit(direction)
            .ok_or_else(|| format!("Unknown direction: {}", direction.as_str()))?;

        let player = self
            .players
            .get_mut(&player_id)
            .ok_or_else(|| "Player does not exist".to_string())?;

        player.move_to(destination);

        Ok(())
    }

    pub fn connect(&mut self, id: ClientId, tx: Sender<String>) {
        let player_id = PlayerId::new(id.value());
        let player_name = format!("Player_{}", player_id.value());

        tx.send("Welcome to the server!\n".to_string()).unwrap();
        let player = Player::new(player_name, RoomId::new(1), tx);

        self.players.insert(player_id, player);
        self.connections.insert(id, player_id);

        println!("Player_{} has connected.", player_id.value());
    }

    pub fn disconnect(&mut self, id: ClientId) {
        let Some(player_id) = self.connections.remove(&id) else {
            return;
        };

        self.players.remove(&player_id);
        println!("Player_{} has disconnected", player_id.value());
    }
}
