use tokio::sync::mpsc::Receiver;

use crate::game::{
    command::CommandContext,
    event::{ClientId, EngineEffect, GameEvent},
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

    pub fn handle(&mut self, event: GameEvent) -> Vec<EngineEffect> {
        let mut effects = Vec::new();
        match event {
            GameEvent::Connected { id } => {
                self.world.connect(id);
                effects.push(EngineEffect::Send {
                    session_id: id,
                    message: "Welcome to the server!\n".to_string(),
                });
            }
            GameEvent::Command { id, text } => self.dispatch(id, &text, &mut effects),
            GameEvent::Disconnected { id } => self.world.disconnect(id),
        }

        effects
    }

    fn dispatch(&mut self, client_id: ClientId, input: &str, effects: &mut Vec<EngineEffect>) {
        let Some(player_id) = self.world.player_id(client_id) else {
            return;
        };

        let Some((command_name, args)) = split_input(input) else {
            effects.push(EngineEffect::Send {
                session_id: client_id,
                message: "Please write a command\n".to_string(),
            });
            return;
        };

        let Some(command) = self.commands.resolve(command_name) else {
            effects.push(EngineEffect::Send {
                session_id: client_id,
                message: format!("Unknown command: {command_name}\n"),
            });
            return;
        };

        let mut context = CommandContext {
            world: &mut self.world,
            registry: &self.commands,
            client_id,
            player_id,
            args,
            effects,
        };

        if let Err(error) = command.execute(&mut context) {
            context.effects.push(EngineEffect::Send {
                session_id: client_id,
                message: format!("Command Error: {error}\n"),
            });
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
