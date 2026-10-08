use std::collections::HashMap;

use crate::game::command::Command;

#[derive(Default)]
pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
    by_name: HashMap<String, usize>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<C>(&mut self, command: C) -> Result<(), String>
    where
        C: Command + 'static,
    {
        let mut keys = Vec::new();

        for raw_key in std::iter::once(command.name()).chain(command.aliases().iter().copied()) {
            let key = raw_key.trim().to_lowercase();

            if key.is_empty() {
                return Err("Command name cannot be empty".to_string());
            }

            if key.chars().any(char::is_whitespace) {
                return Err(format!("Command name cannot contain whitespace: {key}"));
            }

            if self.by_name.contains_key(&key) {
                return Err(format!("Command is already registered: {key}"));
            }

            if !keys.contains(&key) {
                keys.push(key);
            }
        }

        let index = self.commands.len();

        for key in keys {
            self.by_name.insert(key, index);
        }

        self.commands.push(Box::new(command));

        Ok(())
    }

    pub fn resolve(&self, name: &str) -> Option<&dyn Command> {
        let name = name.to_lowercase();
        let index = *self.by_name.get(&name)?;

        Some(self.commands[index].as_ref())
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.commands.iter().map(|command| command.name())
    }
}
