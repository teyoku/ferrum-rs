use crate::{
    error::GameResult,
    hooks::{Effect, GameEvent, Hook},
};

use super::*;

struct TestClient {
    server: ServerHandle,
    session_id: SessionId,
    events: mpsc::Receiver<SessionEvent>,
}

impl TestClient {
    async fn connect(server: &ServerHandle, port: u16) -> Self {
        let (output, events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let address = format!("127.0.0.1:{port}").parse().unwrap();

        let session_id = server.connect(address, output).await.unwrap();

        Self {
            server: server.clone(),
            session_id,
            events,
        }
    }

    async fn input(&self, line: impl Into<String>) {
        self.server
            .input(self.session_id, line.into())
            .await
            .unwrap();
    }

    async fn disconnect(&self) {
        self.server.disconnect(self.session_id).await.unwrap();
    }

    async fn next_text(&mut self) -> String {
        let event = tokio::time::timeout(std::time::Duration::from_secs(5), self.events.recv())
            .await
            .expect("timed out waiting for a session event");

        match event {
            Some(SessionEvent::Text(text)) => text,

            Some(SessionEvent::Close) => {
                panic!("expected text, received close event")
            }

            None => {
                panic!("session event channel was closed")
            }
        }
    }

    async fn expect_text(&mut self, expected: &str) {
        assert_eq!(self.next_text().await, expected);
    }

    async fn login(&mut self, name: &str) -> String {
        self.input(format!("name {name}")).await;

        self.expect_text(&format!("You are now known as {name}.\r\n"))
            .await;

        let room_description = self.next_text().await;

        assert!(
            room_description.contains("Players here:"),
            "expected room description, got: \
             {room_description:?}"
        );

        room_description
    }
}

#[tokio::test]
async fn reports_connected_session_count() {
    let server = ServerActor::spawn();
    let mut client = TestClient::connect(&server, 4000).await;

    client.input("name Alex").await;

    let _ = client.events.recv().await;
    let _ = client.events.recv().await;

    let _ = TestClient::connect(&server, 4001).await;

    client.input("who").await;

    client.expect_text("Players online: (1)\r\nAlex\r\n").await;
}

#[tokio::test]
async fn quit_closes_the_session() {
    let server = ServerActor::spawn();
    let mut client = TestClient::connect(&server, 4000).await;

    client.input("quit").await;

    assert!(matches!(
        client.events.recv().await,
        Some(SessionEvent::Close)
    ));
}

#[tokio::test]
async fn game_commands_require_a_player_name() {
    let server = ServerActor::spawn();
    let mut client = TestClient::connect(&server, 4000).await;

    client.input("look").await;
    client
        .expect_text("Choose a name first: name <player>.\r\n")
        .await;
}

#[tokio::test]
async fn player_can_move_between_rooms() {
    let server = ServerActor::spawn();
    let mut alex = TestClient::connect(&server, 4000).await;
    let mut blair = TestClient::connect(&server, 4001).await;

    alex.login("Alex").await;
    blair.login("Blair").await;
    alex.expect_text("Blair enters the game.\r\n").await;

    alex.input("n").await;
    blair.expect_text("Alex leaves north.\r\n").await;
    assert!(alex.next_text().await.contains("Rusty Tankard"));

    blair.input("north").await;
    alex.expect_text("Blair arrives from south.\r\n").await;
    assert!(blair.next_text().await.contains("Rusty Tankard"));
}

#[tokio::test]
async fn help_only_lists_commands_available_before_login() {
    let server = ServerActor::spawn();
    let mut client = TestClient::connect(&server, 4000).await;

    client.input("help").await;
    let text = client.next_text().await;

    assert!(text.contains("help"));
    assert!(text.contains("name <player>"));
    assert!(text.contains("quit"));

    assert!(!text.contains("look"));
    assert!(!text.contains("say <message>"));
}

#[tokio::test]
async fn help_lists_game_commands_after_login() {
    let server = ServerActor::spawn();
    let mut client = TestClient::connect(&server, 4000).await;

    client.login("Alex").await;
    client.input("help").await;
    let text = client.next_text().await;

    assert!(text.contains("go <direction>"));
    assert!(text.contains("look"));
    assert!(text.contains("say <message>"));
    assert!(text.contains("who"));
}

#[tokio::test]
async fn announces_player_presence_changes() {
    let server = ServerActor::spawn();
    let mut alex = TestClient::connect(&server, 4000).await;
    let mut blair = TestClient::connect(&server, 4001).await;

    alex.login("Alex").await;
    blair.login("Blair").await;
    alex.expect_text("Blair enters the game.\r\n").await;

    blair.input("name Casey").await;
    alex.expect_text("Blair is now known as Casey.\r\n").await;
    blair.expect_text("You are now known as Casey.\r\n").await;
    let _ = blair.next_text().await;

    blair.disconnect().await;
    alex.expect_text("Casey has left the game.\r\n").await;
}

#[tokio::test]
async fn player_can_pick_up_and_drop_an_item() {
    let mut world = World::new("Workshop", "Tools cover a wooden bench.");

    world
        .create_item(world.start_room(), "hammer", "A sturdy iron hammer.")
        .unwrap();

    let server = ServerActor::spawn_with_world(world);
    let mut client = TestClient::connect(&server, 4000).await;

    let room_description = client.login("Alex").await;
    assert!(room_description.contains("Items here: hammer"));

    client.input("examine hammer").await;
    client.expect_text("A sturdy iron hammer.\r\n").await;
    client.input("get hammer").await;
    client.expect_text("You pick up hammer.\r\n").await;
    client.input("x hammer").await;
    client.expect_text("A sturdy iron hammer.\r\n").await;
    client.input("inventory").await;
    client.expect_text("You are carrying: hammer.\r\n").await;
    client.input("drop hammer").await;
    client.expect_text("You drop hammer.\r\n").await;
}

#[tokio::test]
async fn player_can_give_an_item_to_another_player() {
    let mut world = World::new("Workshop", "Tools cover a wooden bench.");

    world
        .create_item(world.start_room(), "iron hammer", "A sturdy iron hammer.")
        .unwrap();

    let server = ServerActor::spawn_with_world(world);
    let mut alex = TestClient::connect(&server, 4000).await;
    let mut blair = TestClient::connect(&server, 4001).await;

    alex.login("Alex").await;
    blair.login("Blair").await;
    alex.expect_text("Blair enters the game.\r\n").await;

    alex.input("get iron hammer").await;
    alex.expect_text("You pick up iron hammer.\r\n").await;
    blair.expect_text("Alex picks up iron hammer.\r\n").await;
    alex.input("give iron hammer Blair").await;
    alex.expect_text("You give iron hammer to Blair.\r\n").await;
    blair.expect_text("Alex gives you iron hammer.\r\n").await;
    blair.input("inventory").await;
    blair
        .expect_text("You are carrying: iron hammer.\r\n")
        .await;
}

#[tokio::test]
async fn renaming_a_player_preserves_their_inventory() {
    let mut world = World::new("Workshop", "Tools cover a wooden bench.");

    world
        .create_item(world.start_room(), "hammer", "A sturdy iron hammer.")
        .unwrap();
    let server = ServerActor::spawn_with_world(world);
    let mut client = TestClient::connect(&server, 4000).await;

    client.login("Alex").await;
    client.input("get hammer").await;
    client.expect_text("You pick up hammer.\r\n").await;
    client.input("name Alicia").await;
    client.expect_text("You are now known as Alicia.\r\n").await;
    let _ = client.next_text().await;
    client.input("inventory").await;
    client.expect_text("You are carrying: hammer.\r\n").await;
}

#[tokio::test]
async fn disconnecting_player_drops_their_inventory() {
    let mut world = World::new("Workshop", "Tools cover a wooden bench.");

    world
        .create_item(world.start_room(), "hammer", "A sturdy iron hammer.")
        .unwrap();

    let server = ServerActor::spawn_with_world(world);
    let mut alex = TestClient::connect(&server, 4000).await;

    alex.login("Alex").await;
    alex.input("get hammer").await;
    alex.expect_text("You pick up hammer.\r\n").await;
    alex.disconnect().await;

    let mut blair = TestClient::connect(&server, 4001).await;
    let room_description = blair.login("Blair").await;
    assert!(room_description.contains("Items here: hammer"));
}

#[tokio::test]
async fn help_describes_a_command_by_its_alias() {
    let server = ServerActor::spawn();
    let mut client = TestClient::connect(&server, 4000).await;

    client.input("help look").await;
    client
        .expect_text("No help available for 'look'.\r\n")
        .await;
    client.login("Alex").await;
    client.input("help x").await;

    let expected = concat!(
        "Command: examine\r\n",
        "Usage: examine <item>\r\n",
        "Aliases: exam, x\r\n",
        "Description: Examine an item nearby or ",
        "in your inventory.\r\n",
    );

    client.expect_text(expected).await;
}

#[derive(Debug)]
struct ClosedGate;

impl Hook for ClosedGate {
    fn before(&self, event: &GameEvent) -> GameResult<Vec<Effect>> {
        match event {
            GameEvent::Move { .. } => Err("the gate is closed".into()),

            _ => Ok(Vec::new()),
        }
    }
}

#[tokio::test]
async fn movement_hook_can_keep_a_player_in_the_current_room() {
    let mut commands = CommandRegistry::new();
    register_builtins(&mut commands);

    let mut hooks = HookRegistry::new();
    hooks.register(ClosedGate);

    let server = ServerActor::spawn_with_registry_hooks_and_world(commands, hooks, World::demo());
    let mut client = TestClient::connect(&server, 4000).await;

    client.login("Alex").await;
    client.input("north").await;
    client
        .expect_text("Command error: the gate is closed\r\n")
        .await;
    client.input("look").await;
    assert!(client.next_text().await.contains("Town Square"));
}

#[derive(Debug)]
struct EchoSpeech;

impl Hook for EchoSpeech {
    fn after(&self, event: &GameEvent) -> Vec<Effect> {
        let GameEvent::Say { room, message, .. } = event else {
            return Vec::new();
        };

        vec![Effect::broadcast(
            *room,
            format!("An echo repeats: {message}"),
        )]
    }
}

#[tokio::test]
async fn speech_hook_can_add_a_room_message() {
    let mut commands = CommandRegistry::new();
    register_builtins(&mut commands);

    let mut hooks = HookRegistry::new();
    hooks.register(EchoSpeech);

    let server = ServerActor::spawn_with_registry_hooks_and_world(commands, hooks, World::demo());
    let mut client = TestClient::connect(&server, 4000).await;

    client.login("Alex").await;
    client.input("say hello").await;
    client.expect_text("You say: hello\r\n").await;
    client.expect_text("An echo repeats: hello\r\n").await;
}
