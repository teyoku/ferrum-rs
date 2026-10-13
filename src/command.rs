use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{
    player::PlayerNameError,
    server::{ServerActor, SessionId},
};

pub type CommandResult = Result<(), &'static str>;

pub trait Command: Debug + Sync + Send {
    fn name(&self) -> &'static str;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn allowed_before_login(&self) -> bool {
        false
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult;
}

pub struct CommandContext<'a> {
    pub session_id: SessionId,
    pub args: &'a str,
    server: &'a mut ServerActor,
}

impl<'a> CommandContext<'a> {
    pub(crate) fn new(session_id: SessionId, args: &'a str, server: &'a mut ServerActor) -> Self {
        Self {
            session_id,
            args,
            server,
        }
    }

    pub fn reply(&mut self, text: impl Into<String>) {
        self.server.send_text(self.session_id, text.into());
    }

    pub fn online_names(&self) -> Vec<String> {
        self.server.online_names()
    }

    pub fn set_name(&mut self, name: &str) -> Result<(), PlayerNameError> {
        self.server.set_player_name(self.session_id, name)
    }

    pub fn say(&mut self, message: &str) -> CommandResult {
        self.server.say(self.session_id, message)
    }

    pub fn look(&mut self) -> CommandResult {
        self.server.look(self.session_id)
    }

    pub fn go(&mut self, direction: &str) -> CommandResult {
        self.server.go(self.session_id, direction)
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
