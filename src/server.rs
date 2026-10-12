use std::{collections::HashMap, error::Error, fmt, net::SocketAddr};

use tokio::sync::{mpsc, oneshot};

use crate::{
    command::{CommandContext, CommandRegistry, CommandResult},
    commands::register_builtins,
    player::{Player, PlayerNameError},
};

pub const SERVER_MAILBOX_CAPACITY: usize = 256;
pub const SESSION_MAILBOX_CAPACITY: usize = 32;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SessionId(u64);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub enum SessionEvent {
    Text(String),
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

#[derive(Debug, Clone)]
pub struct ServerHandle {
    sender: mpsc::Sender<ServerCommand>,
}

#[derive(Debug)]
pub struct ServerActor {
    sessions: HashMap<SessionId, ConnectedSession>,
    next_session_id: u64,
    receiver: mpsc::Receiver<ServerCommand>,
    commands: CommandRegistry,
}

#[derive(Debug)]
pub struct ConnectedSession {
    address: SocketAddr,
    output: mpsc::Sender<SessionEvent>,
    player: Option<Player>,
}

impl ConnectedSession {
    pub fn display_name(&self) -> String {
        self.player
            .as_ref()
            .map(|player| player.name().to_owned())
            .unwrap_or_else(|| self.address.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ServerStopped;

impl fmt::Display for ServerStopped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("server actor has stopped")
    }
}

impl Error for ServerStopped {}

impl ServerActor {
    pub fn spawn() -> ServerHandle {
        let mut commands = CommandRegistry::new();

        register_builtins(&mut commands);

        Self::spawn_with_registry(commands)
    }

    pub fn spawn_with_registry(commands: CommandRegistry) -> ServerHandle {
        let (sender, receiver) = mpsc::channel(SERVER_MAILBOX_CAPACITY);

        let actor = Self {
            sessions: HashMap::new(),
            next_session_id: 1,
            receiver,
            commands,
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
                    self.sessions.remove(&session_id);
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
                player: None,
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

        let mut context = CommandContext::new(session_id, invocation.args, self);

        if let Err(error) = invocation.command.execute(&mut context) {
            context.reply(format!("Command error: {error}\r\n"));
        }
    }

    fn send_to(&mut self, session_id: SessionId, event: SessionEvent) {
        let should_remove = self
            .sessions
            .get(&session_id)
            .is_some_and(|session| session.output.try_send(event).is_err());

        if should_remove {
            self.sessions.remove(&session_id);
        }
    }

    fn broadcast(&mut self, sender_id: SessionId, message: &str) {
        let Some(sender_name) = self
            .sessions
            .get(&sender_id)
            .map(ConnectedSession::display_name)
        else {
            return;
        };

        let mut disconnected = Vec::new();

        for (&session_id, session) in &self.sessions {
            let text = if session_id == sender_id {
                format!("You say: {message}\r\n")
            } else {
                format!("{sender_name} says: {message}\r\n")
            };

            if session.output.try_send(SessionEvent::Text(text)).is_err() {
                disconnected.push(session_id);
            }
        }

        for session_id in disconnected {
            self.sessions.remove(&session_id);
        }
    }

    pub(crate) fn send_text(&mut self, session_id: SessionId, text: String) {
        self.send_to(session_id, SessionEvent::Text(text));
    }

    pub(crate) fn close_session(&mut self, session_id: SessionId) {
        self.send_to(session_id, SessionEvent::Close);
    }

    pub(crate) fn online_count(&self) -> usize {
        self.sessions.len()
    }

    pub(crate) fn broadcast_from(&mut self, session_id: SessionId, message: &str) {
        self.broadcast(session_id, message);
    }

    pub(crate) fn set_player_name(
        &mut self,
        session_id: SessionId,
        name: &str,
    ) -> Result<(), PlayerNameError> {
        let player = Player::new(name)?;
        let normalized_name = player.name().to_lowercase();

        let already_in_use = self.sessions.iter().any(|(&other_id, session)| {
            other_id != session_id
                && session
                    .player
                    .as_ref()
                    .is_some_and(|other| other.name().to_lowercase() == normalized_name)
        });

        if already_in_use {
            return Err(PlayerNameError::AlreadyInUse);
        }

        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(PlayerNameError::SessionNotFound)?;

        session.player = Some(player);

        Ok(())
    }

    pub(crate) fn online_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .sessions
            .values()
            .map(ConnectedSession::display_name)
            .collect();

        names.sort_unstable_by_key(|name| name.to_lowercase());

        names
    }
}

impl ServerHandle {
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

    pub async fn input(&self, session_id: SessionId, line: String) -> Result<(), ServerStopped> {
        self.sender
            .send(ServerCommand::Input { session_id, line })
            .await
            .map_err(|_| ServerStopped)
    }

    pub async fn disconnect(&self, session_id: SessionId) -> Result<(), ServerStopped> {
        self.sender
            .send(ServerCommand::Disconnect { session_id })
            .await
            .map_err(|_| ServerStopped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reports_connected_session_count() {
        let server = ServerActor::spawn();

        let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let address = "127.0.0.1:4000".parse().unwrap();

        let session_id = server.connect(address, output).await.unwrap();

        server.input(session_id, "who".to_owned()).await.unwrap();

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "Players online: 1\r\n"
        ));
    }

    #[tokio::test]
    async fn quit_closes_the_session() {
        let server = ServerActor::spawn();

        let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let address = "127.0.0.1:4000".parse().unwrap();

        let session_id = server.connect(address, output).await.unwrap();

        server.input(session_id, "quit".to_owned()).await.unwrap();

        assert!(matches!(events.recv().await, Some(SessionEvent::Close)));
    }
}
