use crate::game::{
    command::CommandContext,
    commands,
    event::{ClientId, GameEvent},
    registry::CommandRegistry,
    world::World,
};

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