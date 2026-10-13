use std::{collections::HashMap, error::Error, fmt, net::SocketAddr};

use tokio::sync::{mpsc, oneshot};

use crate::{
    command::{CommandContext, CommandRegistry},
    commands::register_builtins,
    player::{Player, PlayerNameError},
    world::{Direction, RoomId, World},
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
    world: World,
}

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

        Self::spawn_with_world(commands, World::demo())
    }

    pub fn spawn_with_registry(commands: CommandRegistry) -> ServerHandle {
        Self::spawn_with_world(commands, World::demo())
    }

    pub fn spawn_with_world(commands: CommandRegistry, world: World) -> ServerHandle {
        let (sender, receiver) = mpsc::channel(SERVER_MAILBOX_CAPACITY);

        let actor = Self {
            sessions: HashMap::new(),
            next_session_id: 1,
            receiver,
            commands,
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

    pub(crate) fn say(&mut self, sender_id: SessionId, message: &str) -> Result<(), &'static str> {
        let Some((sender_name, room)) = self
            .sessions
            .get(&sender_id)
            .and_then(ConnectedSession::player)
            .map(|player| (player.name().to_owned(), player.room()))
        else {
            return Err("choose a name before speaking");
        };

        let mut disconnected = Vec::new();

        for (&session_id, session) in &self.sessions {
            if session.player().map(Player::room) != Some(room) {
                continue;
            }

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

        Ok(())
    }

    pub(crate) fn send_text(&mut self, session_id: SessionId, text: String) {
        self.send_to(session_id, SessionEvent::Text(text));
    }

    pub(crate) fn close_session(&mut self, session_id: SessionId) {
        self.send_to(session_id, SessionEvent::Close);
    }

    pub(crate) fn set_player_name(
        &mut self,
        session_id: SessionId,
        name: &str,
    ) -> Result<(), PlayerNameError> {
        let room = self
            .sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .map(Player::room)
            .unwrap_or_else(|| self.world.start_room());

        let player = Player::new(name, room)?;
        let normalized_name = player.name().to_lowercase();

        let already_in_use = self.sessions.iter().any(|(&other_id, session)| {
            other_id != session_id
                && session
                    .player()
                    .is_some_and(|other| other.name().to_lowercase() == normalized_name)
        });

        if already_in_use {
            return Err(PlayerNameError::AlreadyInUse);
        }

        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(PlayerNameError::SessionNotFound)?;

        session.state = SessionState::Playing(player);

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

    fn current_room(&self, session_id: SessionId) -> Result<RoomId, &'static str> {
        self.sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .map(Player::room)
            .ok_or("choose a name first")
    }

    pub(crate) fn look(&mut self, session_id: SessionId) -> Result<(), &'static str> {
        let room = self.current_room(session_id)?;

        let mut description = self.world.describe(room).ok_or("room does not exist")?;

        let mut players: Vec<_> = self
            .sessions
            .values()
            .filter_map(ConnectedSession::player)
            .filter(|player| player.room() == room)
            .map(|player| player.name().to_owned())
            .collect();

        players.sort_unstable_by_key(|name| name.to_lowercase());

        description.push_str(&format!("Players here: {}\r\n", players.join(", ")));

        self.send_text(session_id, description);

        Ok(())
    }

    pub(crate) fn go(
        &mut self,
        session_id: SessionId,
        direction: &str,
    ) -> Result<(), &'static str> {
        let direction = Direction::parse(direction).ok_or("unknown direction")?;

        let current = self.current_room(session_id)?;

        let destination = self
            .world
            .destination(current, direction)
            .ok_or("there is no exit that way")?;

        let player = self
            .sessions
            .get_mut(&session_id)
            .and_then(ConnectedSession::player_mut)
            .ok_or("choose a name first")?;

        player.move_to(destination);

        self.look(session_id)
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

        server
            .input(session_id, "name Alex".to_owned())
            .await
            .unwrap();

        let _ = events.recv().await;
        let _ = events.recv().await;

        server.input(session_id, "who".to_owned()).await.unwrap();
        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "Players online: (1)\r\nAlex\r\n"
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

    #[tokio::test]
    async fn game_commands_require_a_player_name() {
        let server = ServerActor::spawn();

        let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let address = "127.0.0.1:4000".parse().unwrap();

        let session_id = server.connect(address, output).await.unwrap();

        server.input(session_id, "look".to_owned()).await.unwrap();

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "Choose a name first: name <player>.\r\n"
        ));
    }

    #[tokio::test]
    async fn player_can_move_between_rooms() {
        let server = ServerActor::spawn();

        let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let address = "127.0.0.1:4000".parse().unwrap();

        let session_id = server.connect(address, output).await.unwrap();

        server
            .input(session_id, "name Alex".to_owned())
            .await
            .unwrap();

        let _ = events.recv().await;
        let _ = events.recv().await;

        server
            .input(session_id, "go north".to_owned())
            .await
            .unwrap();

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text)) if text.contains("Rusty Tankard")
        ));
    }
}
