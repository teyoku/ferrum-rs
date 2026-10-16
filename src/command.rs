use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{
    error::GameResult,
    player::PlayerNameError,
    server::{ServerActor, SessionId},
};

/// The result returned by a command implementation.
pub type CommandResult = GameResult;

/// A player command that can be registered with a [`CommandRegistry`].
///
/// Implementations should be stateless. Per-invocation data and controlled
/// access to server operations are provided through [`CommandContext`].
pub trait Command: Debug + Sync + Send {
    /// Returns the canonical command name.
    fn name(&self) -> &'static str;

    /// Returns a short usage string displayed by the help command.
    fn usage(&self) -> &'static str {
        self.name()
    }

    /// Returns a short description displayed by the help command.
    fn description(&self) -> &'static str {
        "No description available."
    }

    /// Returns alternative names accepted by the registry.
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    /// Returns whether this command may run before a player chooses a name.
    fn allowed_before_login(&self) -> bool {
        false
    }

    /// Executes the command for one invocation.
    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult;
}

/// The input and server operations available to a running command.
pub struct CommandContext<'a> {
    /// The session that invoked the command.
    pub session_id: SessionId,
    /// The exact command name or alias used after case normalization.
    pub invoked_as: &'a str,
    /// The trimmed text following the command name.
    pub args: &'a str,
    server: &'a mut ServerActor,
}

impl<'a> CommandContext<'a> {
    pub(crate) fn new(
        session_id: SessionId,
        invoked_as: &'a str,
        args: &'a str,
        server: &'a mut ServerActor,
    ) -> Self {
        Self {
            session_id,
            invoked_as,
            args,
            server,
        }
    }

    /// Sends text to the session that invoked the command.
    pub fn reply(&mut self, text: impl Into<String>) {
        self.server.send_text(self.session_id, text.into());
    }

    /// Returns the sorted display names of all players currently in the game.
    pub fn online_names(&self) -> Vec<String> {
        self.server.online_names()
    }

    /// Chooses or changes the current player's name.
    pub fn set_name(&mut self, name: &str) -> Result<(), PlayerNameError> {
        self.server.set_player_name(self.session_id, name)
    }

    /// Sends a chat message to players in the current room.
    pub fn say(&mut self, message: &str) -> CommandResult {
        self.server.say(self.session_id, message)
    }

    /// Describes the current room to the invoking player.
    pub fn look(&mut self) -> CommandResult {
        self.server.look(self.session_id)
    }

    /// Moves the invoking player in the requested direction.
    pub fn go(&mut self, direction: &str) -> CommandResult {
        self.server.go(self.session_id, direction)
    }

    /// Requests that the invoking session be closed.
    pub fn close(&mut self) {
        self.server.close_session(self.session_id);
    }

    /// Builds general or command-specific help for the invoking session.
    pub fn help_text(&self, command_name: &str) -> String {
        self.server.help_text(self.session_id, command_name)
    }

    /// Moves a named item from the room into the player's inventory.
    pub fn get_item(&mut self, name: &str) -> CommandResult {
        self.server.get_item(self.session_id, name)
    }

    /// Moves a named carried item into the current room.
    pub fn drop_item(&mut self, name: &str) -> CommandResult {
        self.server.drop_item(self.session_id, name)
    }

    /// Sends the invoking player a summary of their inventory.
    pub fn show_inventory(&mut self) -> CommandResult {
        self.server.show_inventory(self.session_id)
    }

    /// Examines a named item in the inventory or current room.
    pub fn examine_item(&mut self, name: &str) -> CommandResult {
        self.server.examine_item(self.session_id, name)
    }

    /// Transfers a carried item to another player in the same room.
    pub fn give_item(&mut self, item_name: &str, recipient_name: &str) -> CommandResult {
        self.server
            .give_item(self.session_id, item_name, recipient_name)
    }
}

/// Maps canonical command names and aliases to command implementations.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
}

/// Metadata used to render help for one command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandHelp {
    /// Canonical command name.
    pub name: &'static str,
    /// Player-facing usage string.
    pub usage: &'static str,
    /// Short player-facing description.
    pub description: &'static str,
    /// Alternative names accepted by the registry.
    pub aliases: &'static [&'static str],
}

impl CommandRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a command and all of its aliases.
    ///
    /// Existing entries with the same case-insensitive names are replaced.
    pub fn register<C>(&mut self, command: C)
    where
        C: Command + 'static,
    {
        self.register_shared(Arc::new(command));
    }

    /// Registers a shared command implementation and all of its aliases.
    ///
    /// This is useful when the caller already stores the command in an [`Arc`].
    pub fn register_shared(&mut self, command: Arc<dyn Command>) {
        let primary_name = command.name().to_lowercase();

        self.commands.insert(primary_name, command.clone());

        for alias in command.aliases() {
            self.commands.insert(alias.to_lowercase(), command.clone());
        }
    }

    /// Resolves the first word of `input` to a command invocation.
    pub fn resolve<'a>(&self, input: &'a str) -> Option<CommandInvocation<'a>> {
        let mut parts = input.trim().splitn(2, char::is_whitespace);

        let invoked_as = parts.next()?.to_lowercase();
        let args = parts.next().unwrap_or("").trim();

        let command = self.commands.get(&invoked_as)?.clone();

        Some(CommandInvocation {
            command,
            invoked_as,
            args,
        })
    }

    /// Returns sorted, de-duplicated help entries visible in the given state.
    ///
    /// When `before_login` is true, commands that require a named player are
    /// omitted.
    pub fn help_entries(&self, before_login: bool) -> Vec<CommandHelp> {
        let mut entries: Vec<_> = self
            .commands
            .values()
            .filter(|command| !before_login || command.allowed_before_login())
            .map(|command| CommandHelp {
                name: command.name(),
                usage: command.usage(),
                description: command.description(),
                aliases: command.aliases(),
            })
            .collect();

        entries.sort_unstable_by_key(|entry| entry.name);
        entries.dedup_by_key(|entry| entry.name);

        entries
    }

    /// Returns help for a command name or alias visible in the given state.
    pub fn help_entry(&self, name: &str, before_login: bool) -> Option<CommandHelp> {
        let normalized_name = name.trim().to_lowercase();
        let command = self.commands.get(&normalized_name)?;

        if before_login && !command.allowed_before_login() {
            return None;
        }

        Some(CommandHelp {
            name: command.name(),
            usage: command.usage(),
            description: command.description(),
            aliases: command.aliases(),
        })
    }
}

/// A resolved command together with its normalized name and arguments.
pub struct CommandInvocation<'a> {
    /// The command implementation to execute.
    pub command: Arc<dyn Command>,
    /// The normalized canonical name or alias used by the player.
    pub invoked_as: String,
    /// The trimmed input following the command name.
    pub args: &'a str,
}
