use crate::game::{commands, event::GameEvent, registry::CommandRegistry, world::World};

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
}
