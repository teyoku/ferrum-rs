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
        Self::spawn_with_world(World::demo())
    }

    pub fn spawn_with_world(world: World) -> ServerHandle {
        let mut commands = CommandRegistry::new();

        register_builtins(&mut commands);

        Self::spawn_with_registry_and_world(commands, world)
    }

    pub fn spawn_with_registry(commands: CommandRegistry) -> ServerHandle {
        Self::spawn_with_registry_and_world(commands, World::demo())
    }

    pub fn spawn_with_registry_and_world(commands: CommandRegistry, world: World) -> ServerHandle {
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
                SessionState::Playing(player) => Some((player.name().to_owned(), player.room())),
            });

        if let Some((player_name, room)) = departing_player {
            self.send_room(room, None, format!("{player_name} has left the game.\r\n"));
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

    pub(crate) fn say(&mut self, sender_id: SessionId, message: &str) -> Result<(), &'static str> {
        let Some((sender_name, room)) = self
            .sessions
            .get(&sender_id)
            .and_then(ConnectedSession::player)
            .map(|player| (player.name().to_owned(), player.room()))
        else {
            return Err("choose a name before speaking");
        };

        self.send_text(sender_id, format!("You say: {message}\r\n"));

        self.send_room(
            room,
            Some(sender_id),
            format!("{sender_name} says: {message}\r\n"),
        );

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
        let previous_payer = self
            .sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .map(|player| (player.name().to_owned(), player.room()));

        let room = previous_payer
            .as_ref()
            .map(|(_, room)| *room)
            .unwrap_or_else(|| self.world.start_room());

        let player = Player::new(name, room)?;
        let player_name = player.name().to_owned();
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

        let announcement = match previous_payer {
            Some((previous_name, _)) if previous_name != player_name => Some(format!(
                "{previous_name} is now known as {player_name}.\r\n"
            )),
            Some(_) => None,
            None => Some(format!("{player_name} enters the game.\r\n")),
        };

        if let Some(announcement) = announcement {
            self.send_room(room, Some(session_id), announcement);
        }

        Ok(())
    }

    pub(crate) fn online_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .sessions
            .values()
            .filter_map(ConnectedSession::player)
            .map(|player| player.name().to_owned())
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

        let (current, player_name) = self
            .sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .map(|player| (player.room(), player.name().to_owned()))
            .ok_or("choose a name first")?;

        let destination = self
            .world
            .destination(current, direction)
            .ok_or("there is no exit that way")?;

        self.send_room(
            current,
            Some(session_id),
            format!("{player_name} leaves {}.\r\n", direction.as_str()),
        );

        let player = self
            .sessions
            .get_mut(&session_id)
            .and_then(ConnectedSession::player_mut)
            .ok_or("choose a name first")?;

        player.move_to(destination);

        self.send_room(
            destination,
            Some(session_id),
            format!(
                "{player_name} arrives from {}.\r\n",
                direction.opposite().as_str()
            ),
        );

        self.look(session_id)
    }

    pub(crate) fn help_text(&self, session_id: SessionId) -> String {
        let before_login = !self
            .sessions
            .get(&session_id)
            .is_some_and(ConnectedSession::is_playing);

        let entries = self.commands.help_entries(before_login);

        let mut text = String::from("Commands:\r\n");

        for entry in entries {
            text.push_str(&format!("  {:<18} {}\r\n", entry.usage, entry.description));
        }

        text
    }

    pub(crate) fn get_item(
        &mut self,
        session_id: SessionId,
        requested_name: &str,
    ) -> Result<(), &'static str> {
        let (player_name, room) = self
            .sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .map(|player| (player.name().to_owned(), player.room()))
            .ok_or("choose a name first")?;

        let item = self
            .world
            .take_item(room, requested_name)
            .ok_or("that item is not here")?;

        let item_name = item.name().to_owned();

        self.sessions
            .get_mut(&session_id)
            .and_then(ConnectedSession::player_mut)
            .expect("player was checked above")
            .give_item(item);

        self.send_text(session_id, format!("You pick up {item_name}.\r\n"));

        self.send_room(
            room,
            Some(session_id),
            format!("{player_name} picks up {item_name}.\r\n"),
        );

        Ok(())
    }

    pub(crate) fn drop_item(
        &mut self,
        session_id: SessionId,
        requested_name: &str,
    ) -> Result<(), &'static str> {
        let (player_name, room, item) = {
            let player = self
                .sessions
                .get_mut(&session_id)
                .and_then(ConnectedSession::player_mut)
                .ok_or("choose a name first")?;

            let item = player
                .take_item(requested_name)
                .ok_or("you are not carrying that item")?;

            (player.name().to_owned(), player.room(), item)
        };

        let item_name = item.name().to_owned();

        self.world
            .add_item(room, item)
            .map_err(|_| "room does not exist")?;

        self.send_text(session_id, format!("You drop {item_name}.\r\n"));

        self.send_room(
            room,
            Some(session_id),
            format!("{player_name} drops {item_name}.\r\n"),
        );

        Ok(())
    }

    pub(crate) fn show_inventory(&mut self, session_id: SessionId) -> Result<(), &'static str> {
        let player = self
            .sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .ok_or("choose a name first")?;

        let items = if player.inventory().is_empty() {
            "nothing".to_owned()
        } else {
            player
                .inventory()
                .iter()
                .map(|item| item.name())
                .collect::<Vec<_>>()
                .join(", ")
        };

        self.send_text(session_id, format!("You are carrying: {items}.\r\n"));

        Ok(())
    }

    pub(crate) fn examine_item(
        &mut self,
        session_id: SessionId,
        requested_name: &str,
    ) -> Result<(), &'static str> {
        let (room, carried_description) = self
            .sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .map(|player| {
                let description = player
                    .find_item(requested_name)
                    .map(|item| item.description().to_owned());

                (player.room(), description)
            })
            .ok_or("choose a name first")?;

        let description = carried_description
            .or_else(|| {
                self.world
                    .find_item(room, requested_name)
                    .map(|item| item.description().to_owned())
            })
            .ok_or("you do not see that item")?;

        self.send_text(session_id, format!("{description}\r\n"));

        Ok(())
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

        let (anonymous_output, _anonymous_events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let anonymous_address = "127.0.0.1:4001".parse().unwrap();

        server
            .connect(anonymous_address, anonymous_output)
            .await
            .unwrap();

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

        let (alex_output, mut alex_events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let alex_address = "127.0.0.1:4000".parse().unwrap();

        let alex_id = server.connect(alex_address, alex_output).await.unwrap();

        let (blair_output, mut blair_events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let blair_address = "127.0.0.1:4001".parse().unwrap();

        let blair_id = server.connect(blair_address, blair_output).await.unwrap();

        server.input(alex_id, "name Alex".to_owned()).await.unwrap();

        let _ = alex_events.recv().await;
        let _ = alex_events.recv().await;

        server
            .input(blair_id, "name Blair".to_owned())
            .await
            .unwrap();

        let _ = blair_events.recv().await;
        let _ = blair_events.recv().await;
        let _ = alex_events.recv().await;

        server.input(alex_id, "n".to_owned()).await.unwrap();

        assert!(matches!(blair_events.recv().await,
            Some(SessionEvent::Text(text)) if text == "Alex leaves north.\r\n"));

        assert!(matches!(alex_events.recv().await,
        Some(SessionEvent::Text(text))
            if text.contains("Rusty Tankard")));

        server.input(blair_id, "north".to_owned()).await.unwrap();

        assert!(matches!(
            alex_events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "Blair arrives from south.\r\n"
        ));

        assert!(matches!(
            blair_events.recv().await,
            Some(SessionEvent::Text(text))
                if text.contains("Rusty Tankard")
        ));
    }

    #[tokio::test]
    async fn help_only_lists_commands_available_before_login() {
        let server = ServerActor::spawn();

        let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let address = "127.0.0.1:4000".parse().unwrap();

        let session_id = server.connect(address, output).await.unwrap();

        server.input(session_id, "help".to_owned()).await.unwrap();

        let Some(SessionEvent::Text(text)) = events.recv().await else {
            panic!("expected help text");
        };

        assert!(text.contains("help"));
        assert!(text.contains("name <player>"));
        assert!(text.contains("quit"));

        assert!(!text.contains("look"));
        assert!(!text.contains("say <message>"));
    }

    #[tokio::test]
    async fn help_lists_game_commands_after_login() {
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

        server.input(session_id, "help".to_owned()).await.unwrap();

        let Some(SessionEvent::Text(text)) = events.recv().await else {
            panic!("expected help text");
        };

        assert!(text.contains("go <direction>"));
        assert!(text.contains("look"));
        assert!(text.contains("say <message>"));
        assert!(text.contains("who"));
    }

    #[tokio::test]
    async fn announces_player_presence_changes() {
        let server = ServerActor::spawn();

        let (alex_output, mut alex_events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let alex_id = server
            .connect("127.0.0.1:4000".parse().unwrap(), alex_output)
            .await
            .unwrap();

        let (blair_output, mut blair_events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let blair_id = server
            .connect("127.0.0.1:4001".parse().unwrap(), blair_output)
            .await
            .unwrap();

        server.input(alex_id, "name Alex".to_owned()).await.unwrap();

        let _ = alex_events.recv().await;
        let _ = alex_events.recv().await;

        server
            .input(blair_id, "name Blair".to_owned())
            .await
            .unwrap();

        assert!(matches!(
            alex_events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "Blair enters the game.\r\n"
        ));

        let _ = blair_events.recv().await;
        let _ = blair_events.recv().await;

        server
            .input(blair_id, "name Casey".to_owned())
            .await
            .unwrap();

        assert!(matches!(
            alex_events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "Blair is now known as Casey.\r\n"
        ));

        let _ = blair_events.recv().await;
        let _ = blair_events.recv().await;

        server.disconnect(blair_id).await.unwrap();

        assert!(matches!(
            alex_events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "Casey has left the game.\r\n"
        ));
    }

    #[tokio::test]
    async fn player_can_pick_up_and_drop_an_item() {
        let mut world = World::new("Workshop", "Tools cover a wooden bench.");

        world
            .add_item(
                world.start_room(),
                crate::Item::new("hammer", "A sturdy iron hammer."),
            )
            .unwrap();

        let server = ServerActor::spawn_with_world(world);

        let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let session_id = server
            .connect("127.0.0.1:4000".parse().unwrap(), output)
            .await
            .unwrap();

        server
            .input(session_id, "name Alex".to_owned())
            .await
            .unwrap();

        let _ = events.recv().await;

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text.contains("Items here: hammer")
        ));

        server
            .input(session_id, "examine hammer".to_owned())
            .await
            .unwrap();

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "A sturdy iron hammer.\r\n"
        ));

        server
            .input(session_id, "get hammer".to_owned())
            .await
            .unwrap();

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "You pick up hammer.\r\n"
        ));

        server
            .input(session_id, "inventory".to_owned())
            .await
            .unwrap();

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "You are carrying: hammer.\r\n"
        ));

        server
            .input(session_id, "drop hammer".to_owned())
            .await
            .unwrap();

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "You drop hammer.\r\n"
        ));
    }
}
