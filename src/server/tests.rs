use crate::hooks::Hook;

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
        .input(session_id, "x hammer".to_owned())
        .await
        .unwrap();

    assert!(matches!(
        events.recv().await,
        Some(SessionEvent::Text(text))
            if text == "A sturdy iron hammer.\r\n"
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

#[tokio::test]
async fn player_can_give_an_item_to_another_player() {
    let mut world = World::new("Workshop", "Tools cover a wooden bench.");

    world
        .add_item(
            world.start_room(),
            crate::Item::new("iron hammer", "A sturdy iron hammer."),
        )
        .unwrap();

    let server = ServerActor::spawn_with_world(world);

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

    let _ = blair_events.recv().await;
    let _ = blair_events.recv().await;
    let _ = alex_events.recv().await;

    server
        .input(alex_id, "get iron hammer".to_owned())
        .await
        .unwrap();

    let _ = alex_events.recv().await;
    let _ = blair_events.recv().await;

    server
        .input(alex_id, "give iron hammer Blair".to_owned())
        .await
        .unwrap();

    assert!(matches!(
        alex_events.recv().await,
        Some(SessionEvent::Text(text))
            if text
                == "You give iron hammer to Blair.\r\n"
    ));

    assert!(matches!(
        blair_events.recv().await,
        Some(SessionEvent::Text(text))
            if text
                == "Alex gives you iron hammer.\r\n"
    ));

    server
        .input(blair_id, "inventory".to_owned())
        .await
        .unwrap();

    assert!(matches!(
        blair_events.recv().await,
        Some(SessionEvent::Text(text))
            if text
                == "You are carrying: iron hammer.\r\n"
    ));
}

#[tokio::test]
async fn renaming_a_player_preserves_their_inventory() {
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
    let _ = events.recv().await;

    server
        .input(session_id, "get hammer".to_owned())
        .await
        .unwrap();

    let _ = events.recv().await;

    server
        .input(session_id, "name Alicia".to_owned())
        .await
        .unwrap();

    let _ = events.recv().await;
    let _ = events.recv().await;

    server
        .input(session_id, "inventory".to_owned())
        .await
        .unwrap();

    assert!(matches!(
        events.recv().await,
        Some(SessionEvent::Text(text))
            if text
                == "You are carrying: hammer.\r\n"
    ));
}

#[tokio::test]
async fn disconnecting_player_drops_their_inventory() {
    let mut world = World::new("Workshop", "Tools cover a wooden bench.");

    world
        .add_item(
            world.start_room(),
            crate::Item::new("hammer", "A sturdy iron hammer."),
        )
        .unwrap();

    let server = ServerActor::spawn_with_world(world);

    let (alex_output, mut alex_events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

    let alex_id = server
        .connect("127.0.0.1:4000".parse().unwrap(), alex_output)
        .await
        .unwrap();

    server.input(alex_id, "name Alex".to_owned()).await.unwrap();

    let _ = alex_events.recv().await;
    let _ = alex_events.recv().await;

    server
        .input(alex_id, "get hammer".to_owned())
        .await
        .unwrap();

    let _ = alex_events.recv().await;

    server.disconnect(alex_id).await.unwrap();

    let (blair_output, mut blair_events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

    let blair_id = server
        .connect("127.0.0.1:4001".parse().unwrap(), blair_output)
        .await
        .unwrap();

    server
        .input(blair_id, "name Blair".to_owned())
        .await
        .unwrap();

    let _ = blair_events.recv().await;

    assert!(matches!(
        blair_events.recv().await,
        Some(SessionEvent::Text(text))
            if text.contains("Items here: hammer")
    ));
}

#[tokio::test]
async fn help_describes_a_command_by_its_alias() {
    let server = ServerActor::spawn();

    let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

    let session_id = server
        .connect("127.0.0.1:4000".parse().unwrap(), output)
        .await
        .unwrap();

    server
        .input(session_id, "help look".to_owned())
        .await
        .unwrap();

    assert!(matches!(
        events.recv().await,
        Some(SessionEvent::Text(text))
            if text
                == "No help available for 'look'.\r\n"
    ));

    server
        .input(session_id, "name Alex".to_owned())
        .await
        .unwrap();

    let _ = events.recv().await;
    let _ = events.recv().await;

    server.input(session_id, "help x".to_owned()).await.unwrap();

    let expected = concat!(
        "Command: examine\r\n",
        "Usage: examine <item>\r\n",
        "Aliases: exam, x\r\n",
        "Description: Examine an item nearby or ",
        "in your inventory.\r\n",
    );

    assert!(matches!(
        events.recv().await,
        Some(SessionEvent::Text(text))
            if text == expected
    ));
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
    let _ = events.recv().await;

    server.input(session_id, "north".to_owned()).await.unwrap();

    assert!(matches!(
        events.recv().await,
        Some(SessionEvent::Text(text))
            if text == "Command error: the gate is closed\r\n"
    ));

    server.input(session_id, "look".to_owned()).await.unwrap();

    assert!(matches!(
        events.recv().await,
        Some(SessionEvent::Text(text))
            if text.contains("Town Square")
    ));
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
    let _ = events.recv().await;

    server
        .input(session_id, "say hello".to_owned())
        .await
        .unwrap();

    assert!(matches!(
        events.recv().await,
        Some(SessionEvent::Text(text))
            if text == "You say: hello\r\n"
    ));

    assert!(matches!(
        events.recv().await,
        Some(SessionEvent::Text(text))
            if text == "An echo repeats: hello\r\n"
    ));
}
