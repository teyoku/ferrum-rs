use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};

use crate::game::{
    command::Command,
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
            GameEvent::Connected { id, tx } => self.connect(id, tx),
            GameEvent::Command { id, text } => self.command(id, text),
            GameEvent::Disconnected { id } => self.disconnect(id),
            GameEvent::Shutdown => todo!(),
        }
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

    pub fn command(&mut self, id: ClientId, text: String) {
        let Some(&player_id) = self.connections.get(&id) else {
            println!("Command from unknown client.");
            return;
        };

        let player = self.players.get_mut(&player_id).unwrap();
        let tx = player.tx();

        let room_id = player.room();
        let room = self.rooms.get_mut(&room_id).unwrap();

        match Command::parse(&text) {
            Ok(command) => match command {
                Command::Look => {
                    let exits_str = room
                        .exits()
                        .keys()
                        .map(|dir| dir.as_str())
                        .collect::<Vec<&str>>()
                        .join(", ");

                    let msg = format!(
                        "{}\n{}\n\n[Exits: {}]\n",
                        room.title(),
                        room.description(),
                        exits_str
                    );
                    tx.send(msg).unwrap();
                }
                Command::Move(direction) => {
                    let Some(exit_id) = room.exit(direction) else {
                        tx.send(format!("Unknown direction: {}", direction.as_str()))
                            .unwrap();
                        return;
                    };

                    player.move_to(exit_id);
                }
                Command::Say(msg) => tx.send(format!("You said \"{msg}\"")).unwrap(),
                Command::Help => todo!(),
                Command::Quit => todo!(),
            },
            Err(err) => tx.send(format!("Command Error: {err}")).unwrap(),
        }
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
