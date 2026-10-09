use tokio::sync::mpsc::Receiver;

use crate::game::{
    command::CommandContext,
    event::{ClientId, GameEvent},
    registry::CommandRegistry,
    world::World,
};

pub mod command;
pub mod commands;
pub mod event;
pub mod player;
pub mod registry;
pub mod room;
pub mod world;

pub struct Game {
    world: World,
    commands: CommandRegistry,
}

impl Game {
    pub fn new() -> Self {
        let mut command_registry = CommandRegistry::new();

        commands::register_all(&mut command_registry).expect("Cannot register built-in commands");

        Self {
            world: World::new(),
            commands: command_registry,
        }
    }

    pub fn handle(&mut self, event: GameEvent) {
        match event {
            GameEvent::Connected { id, tx } => self.world.connect(id, tx),
            GameEvent::Command { id, text } => self.dispatch(id, &text),
            GameEvent::Disconnected { id } => self.world.disconnect(id),
        }
    }

    fn dispatch(&mut self, client_id: ClientId, input: &str) {
        let Some(player_id) = self.world.player_id(client_id) else {
            return;
        };

        let Some((command_name, args)) = split_input(input) else {
            let _ = self.world.send_to(player_id, "Please write a command\n");
            return;
        };

        let Some(command) = self.commands.resolve(command_name) else {
            let _ = self
                .world
                .send_to(player_id, format!("Unknown command: {command_name}\n"));
            return;
        };

        let mut context = CommandContext {
            world: &mut self.world,
            registry: &self.commands,
            client_id,
            player_id,
            args,
        };

        if let Err(error) = command.execute(&mut context) {
            let _ = self
                .world
                .send_to(player_id, format!("Command Error: {error}\n"));
        }
    }
}

fn split_input(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();

    if input.is_empty() {
        return None;
    }

    match input.find(char::is_whitespace) {
        Some(index) => {
            let (command, args) = input.split_at(index);
            Some((command, args.trim()))
        }
        None => Some((input, "")),
    }
}

pub async fn run_game_world(mut event_rx: Receiver<GameEvent>) {
    let mut game = Game::new();

    while let Some(event) = event_rx.recv().await {
        game.handle(event);
    }
}
