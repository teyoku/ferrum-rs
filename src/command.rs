use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{ServerActor, server::SessionId};

pub type CommandResult = Result<(), &'static str>;

pub trait Command: Debug + Sync + Send {
    fn name(&self) -> &'static str;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult;
}

pub struct CommandContext<'a> {
    pub session_id: SessionId,
    pub args: &'a str,
    server: &'a mut ServerActor,
}

impl<'a> CommandContext<'a> {
    pub fn new(session_id: SessionId, args: &'a str, server: &'a mut ServerActor) -> Self {
        Self {
            session_id,
            args,
            server,
        }
    }

    pub fn reply(&mut self, text: impl Into<String>) {
        self.server.send_text(self.session_id, text.into());
    }

    pub fn online_count(&self) -> usize {
        self.server.online_count()
    }

    pub fn say(&mut self, message: &str) {
        self.server.broadcast_from(self.session_id, message);
    }

    pub fn close(&mut self) {
        self.server.close_session(self.session_id);
    }
}

#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<C>(&mut self, command: C)
    where
        C: Command + 'static,
    {
        self.register_shared(Arc::new(command));
    }

    pub fn register_shared(&mut self, command: Arc<dyn Command>) {
        let primary_name = command.name().to_lowercase();

        self.commands.insert(primary_name, command.clone());

        for alias in command.aliases() {
            self.commands.insert(alias.to_lowercase(), command.clone());
        }
    }

    pub fn resolve<'a>(&self, input: &'a str) -> Option<CommandInvocation<'a>> {
        let mut parts = input.trim().splitn(2, char::is_whitespace);

        let name = parts.next()?.to_lowercase();
        let args = parts.next().unwrap_or("").trim();
        let command = self.commands.get(&name)?.clone();

        Some(CommandInvocation { command, args })
    }
}

pub struct CommandInvocation<'a> {
    pub command: Arc<dyn Command>,
    pub args: &'a str,
}
