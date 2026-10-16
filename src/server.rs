use std::{collections::HashMap, error::Error, fmt, net::SocketAddr};

use tokio::sync::{mpsc, oneshot};

use crate::{
    command::{CommandContext, CommandRegistry},
    commands::register_builtins,
    hooks::HookRegistry,
    player::Player,
    world::{RoomId, World},
};

mod actions;

/// Maximum number of commands buffered for the server actor.
pub const SERVER_MAILBOX_CAPACITY: usize = 256;
/// Maximum number of outbound events buffered for one client session.
pub const SESSION_MAILBOX_CAPACITY: usize = 32;

/// Identifies one connection for the lifetime of a server actor.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SessionId(u64);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An outbound event consumed by a client session task.
#[derive(Debug)]
pub enum SessionEvent {
    /// Writes text to the connected client.
    Text(String),
    /// Requests a clean connection shutdown.
    Close,
}

enum ServerCommand {
    Connect {
        address: SocketAddr,
        output: mpsc::Sender<SessionEvent>,
        reply: oneshot::Sender<SessionId>,
    },
    Input {
        session_id: SessionId,
        line: String,
    },
    Disconnect {
        session_id: SessionId,
    },
}

/// A clonable, asynchronous command channel for a running [`ServerActor`].
#[derive(Debug, Clone)]
pub struct ServerHandle {
    sender: mpsc::Sender<ServerCommand>,
}

/// Owns the world and serializes all connected-player state changes.
///
/// Create a running actor with one of the `spawn` methods. Callers interact
/// with it through the returned [`ServerHandle`].
#[derive(Debug)]
pub struct ServerActor {
    sessions: HashMap<SessionId, ConnectedSession>,
    next_session_id: u64,
    receiver: mpsc::Receiver<ServerCommand>,
    commands: CommandRegistry,
    hooks: HookRegistry,
    world: World,
}

/// Server-side state associated with one connected client.
#[derive(Debug)]
pub struct ConnectedSession {
    address: SocketAddr,
    output: mpsc::Sender<SessionEvent>,
    state: SessionState,
}

#[derive(Debug)]
enum SessionState {
    AwaitingName,
    Playing(Player),
}

impl ConnectedSession {
    /// Returns the player name, or the peer address before login.
    pub fn display_name(&self) -> String {
        self.player()
            .map(|player| player.name().to_owned())
            .unwrap_or_else(|| self.address.to_string())
    }

    fn player(&self) -> Option<&Player> {
        match &self.state {
            SessionState::AwaitingName => None,
            SessionState::Playing(player) => Some(player),
        }
    }

    fn player_mut(&mut self) -> Option<&mut Player> {
        match &mut self.state {
            SessionState::AwaitingName => None,
            SessionState::Playing(player) => Some(player),
        }
    }

    fn is_playing(&self) -> bool {
        matches!(self.state, SessionState::Playing(_))
    }
}

/// Indicates that a [`ServerHandle`] can no longer reach its actor task.
#[derive(Debug, Clone, Copy)]
pub struct ServerStopped;

impl fmt::Display for ServerStopped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("server actor has stopped")
    }
}

impl Error for ServerStopped {}

impl ServerActor {
    /// Spawns a server actor with the built-in commands and demo world.
    ///
    /// This method must be called from within a Tokio runtime.
    pub fn spawn() -> ServerHandle {
        Self::spawn_with_world(World::demo())
    }

    /// Spawns a server actor with the built-in commands and the supplied world.
    ///
    /// This method must be called from within a Tokio runtime.
    pub fn spawn_with_world(world: World) -> ServerHandle {
        let mut commands = CommandRegistry::new();

        register_builtins(&mut commands);

        Self::spawn_with_registry_and_world(commands, world)
    }

    /// Spawns a server actor with a custom command registry and demo world.
    ///
    /// This method must be called from within a Tokio runtime.
    pub fn spawn_with_registry(commands: CommandRegistry) -> ServerHandle {
        Self::spawn_with_registry_and_world(commands, World::demo())
    }

    /// Spawns a server actor with custom commands and world, but no hooks.
    ///
    /// This method must be called from within a Tokio runtime.
    pub fn spawn_with_registry_and_world(commands: CommandRegistry, world: World) -> ServerHandle {
        Self::spawn_with_registry_hooks_and_world(commands, HookRegistry::new(), world)
    }

    /// Spawns a fully configured server actor.
    ///
    /// This method must be called from within a Tokio runtime. The returned
    /// handle keeps the actor's command channel alive.
    pub fn spawn_with_registry_hooks_and_world(
        commands: CommandRegistry,
        hooks: HookRegistry,
        world: World,
    ) -> ServerHandle {
        let (sender, receiver) = mpsc::channel(SERVER_MAILBOX_CAPACITY);

        let actor = Self {
            sessions: HashMap::new(),
            next_session_id: 1,
            receiver,
            commands,
            hooks,
            world,
        };

        tokio::spawn(actor.run());

        ServerHandle { sender }
    }

    async fn run(mut self) {
        while let Some(command) = self.receiver.recv().await {
            match command {
                ServerCommand::Connect {
                    address,
                    output,
                    reply,
                } => self.connect(address, output, reply),
                ServerCommand::Input { session_id, line } => {
                    self.handle_input(session_id, &line);
                }
                ServerCommand::Disconnect { session_id } => {
                    self.disconnect(session_id);
                }
            }
        }
    }

    fn connect(
        &mut self,
        address: SocketAddr,
        output: mpsc::Sender<SessionEvent>,
        reply: oneshot::Sender<SessionId>,
    ) {
        let session_id = SessionId(self.next_session_id);

        self.next_session_id = self
            .next_session_id
            .checked_add(1)
            .expect("session identifier overflowed");

        self.sessions.insert(
            session_id,
            ConnectedSession {
                address,
                output,
                state: SessionState::AwaitingName,
            },
        );

        if reply.send(session_id).is_err() {
            self.sessions.remove(&session_id);
        }
    }

    fn handle_input(&mut self, session_id: SessionId, line: &str) {
        let Some(invocation) = self.commands.resolve(line) else {
            self.send_text(session_id, "Unknown command.\r\n".to_owned());
            return;
        };

        let is_playing = self
            .sessions
            .get(&session_id)
            .is_some_and(ConnectedSession::is_playing);

        if !is_playing && !invocation.command.allowed_before_login() {
            self.send_text(
                session_id,
                "Choose a name first: name <player>.\r\n".to_owned(),
            );

            return;
        }

        let mut context =
            CommandContext::new(session_id, &invocation.invoked_as, invocation.args, self);

        if let Err(error) = invocation.command.execute(&mut context) {
            context.reply(format!("Command error: {error}\r\n"));
        }
    }

    fn disconnect(&mut self, session_id: SessionId) {
        let departing_player = self
            .sessions
            .remove(&session_id)
            .and_then(|session| match session.state {
                SessionState::AwaitingName => None,
                SessionState::Playing(player) => {
                    let player_name = player.name().to_owned();
                    let room = player.room();
                    let inventory = player.into_inventory();

                    Some((player_name, room, inventory))
                }
            });

        if let Some((player_name, room, inventory)) = departing_player {
            let dropped_items = !inventory.is_empty();

            // A disconnect must not destroy owned items; return them to the
            // last valid room before announcing the departure.
            for item in inventory {
                self.world
                    .add_item(room, item)
                    .expect("player room must exist");
            }

            let announcement = if dropped_items {
                format!(
                    "{player_name} has left the game, \
                 dropping everything they carried.\r\n"
                )
            } else {
                format!("{player_name} has left the game.\r\n")
            };

            self.send_room(room, None, announcement);
        }
    }

    fn send_to(&mut self, session_id: SessionId, event: SessionEvent) {
        // A full or closed session mailbox means the network task is no longer
        // keeping up. Removing it also performs the normal disconnect cleanup.
        let should_remove = self
            .sessions
            .get(&session_id)
            .is_some_and(|session| session.output.try_send(event).is_err());

        if should_remove {
            self.disconnect(session_id);
        }
    }

    fn send_room(&mut self, room: RoomId, except: Option<SessionId>, text: impl Into<String>) {
        let recipients: Vec<_> = self
            .sessions
            .iter()
            .filter(|(session_id, _)| Some(**session_id) != except)
            .filter(|(_, session)| session.player().map(Player::room) == Some(room))
            .map(|(session_id, _)| *session_id)
            .collect();

        let text = text.into();

        for session_id in recipients {
            self.send_to(session_id, SessionEvent::Text(text.clone()));
        }
    }

    pub(crate) fn send_text(&mut self, session_id: SessionId, text: String) {
        self.send_to(session_id, SessionEvent::Text(text));
    }

    pub(crate) fn close_session(&mut self, session_id: SessionId) {
        self.send_to(session_id, SessionEvent::Close);
    }
}

impl ServerHandle {
    /// Registers a newly accepted connection with the actor.
    ///
    /// The `output` channel receives text and close events for the connection.
    ///
    /// # Errors
    ///
    /// Returns [`ServerStopped`] when the actor task is no longer running.
    pub async fn connect(
        &self,
        address: SocketAddr,
        output: mpsc::Sender<SessionEvent>,
    ) -> Result<SessionId, ServerStopped> {
        let (reply, response) = oneshot::channel();

        self.sender
            .send(ServerCommand::Connect {
                address,
                output,
                reply,
            })
            .await
            .map_err(|_| ServerStopped)?;

        response.await.map_err(|_| ServerStopped)
    }

    /// Submits one complete input line for a session.
    ///
    /// # Errors
    ///
    /// Returns [`ServerStopped`] when the actor task is no longer running.
    pub async fn input(&self, session_id: SessionId, line: String) -> Result<(), ServerStopped> {
        self.sender
            .send(ServerCommand::Input { session_id, line })
            .await
            .map_err(|_| ServerStopped)
    }

    /// Notifies the actor that a session has disconnected.
    ///
    /// # Errors
    ///
    /// Returns [`ServerStopped`] when the actor task is no longer running.
    pub async fn disconnect(&self, session_id: SessionId) -> Result<(), ServerStopped> {
        self.sender
            .send(ServerCommand::Disconnect { session_id })
            .await
            .map_err(|_| ServerStopped)
    }
}

#[cfg(test)]
mod tests;
