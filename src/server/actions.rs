use super::{ConnectedSession, ServerActor, SessionId, SessionState};

use crate::{
    error::GameResult,
    hooks::{Effect, EffectKind, GameEvent},
    player::{Player, PlayerNameError},
    world::{Direction, RoomId},
};

impl ServerActor {
    pub(crate) fn say(&mut self, sender_id: SessionId, message: &str) -> GameResult {
        let Some((sender_name, room)) = self
            .sessions
            .get(&sender_id)
            .and_then(ConnectedSession::player)
            .map(|player| (player.name().to_owned(), player.room()))
        else {
            return Err("choose a name before speaking".into());
        };

        let event = GameEvent::Say {
            player_name: sender_name.clone(),
            room,
            message: message.to_owned(),
        };

        let effects = self.hooks.before(&event)?;

        self.apply_hook_effects(sender_id, effects);

        self.send_text(sender_id, format!("You say: {message}\r\n"));

        self.send_room(
            room,
            Some(sender_id),
            format!("{sender_name} says: {message}\r\n"),
        );

        let effects = self.hooks.after(&event);

        self.apply_hook_effects(sender_id, effects);

        Ok(())
    }

    pub(crate) fn set_player_name(
        &mut self,
        session_id: SessionId,
        name: &str,
    ) -> Result<(), PlayerNameError> {
        let previous_player = self
            .sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .map(|player| (player.name().to_owned(), player.room()));

        let room = previous_player
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

        match &mut session.state {
            SessionState::AwaitingName => {
                session.state = SessionState::Playing(player);
            }
            SessionState::Playing(current_player) => {
                current_player.rename(&player_name)?;
            }
        }

        let announcement = match previous_player {
            Some((previous_name, _)) if previous_name != player_name => Some(format!(
                "{previous_name} is now known as \
             {player_name}.\r\n"
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

    fn current_room(&self, session_id: SessionId) -> GameResult<RoomId> {
        self.sessions
            .get(&session_id)
            .and_then(ConnectedSession::player)
            .map(Player::room)
            .ok_or("choose a name first".into())
    }

    pub(crate) fn look(&mut self, session_id: SessionId) -> GameResult {
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

    pub(crate) fn go(&mut self, session_id: SessionId, direction: &str) -> GameResult {
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

        let event = GameEvent::Move {
            player_name: player_name.clone(),
            from: current,
            to: destination,
            direction,
        };

        let effects = self.hooks.before(&event)?;

        self.apply_hook_effects(session_id, effects);

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

        let effects = self.hooks.after(&event);

        self.apply_hook_effects(session_id, effects);

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

    pub(crate) fn help_text(&self, session_id: SessionId, command_name: &str) -> String {
        let before_login = !self
            .sessions
            .get(&session_id)
            .is_some_and(ConnectedSession::is_playing);

        if !command_name.is_empty() {
            let Some(entry) = self.commands.help_entry(command_name, before_login) else {
                return format!("No help available for '{command_name}'.\r\n");
            };

            let aliases = if entry.aliases.is_empty() {
                "none".to_owned()
            } else {
                entry.aliases.join(", ")
            };

            return format!(
                "Command: {}\r\n\
                 Usage: {}\r\n\
                 Aliases: {}\r\n\
                 Description: {}\r\n",
                entry.name, entry.usage, aliases, entry.description
            );
        }

        let entries = self.commands.help_entries(before_login);
        let mut text = String::from("Commands:\r\n");

        for entry in entries {
            text.push_str(&format!("  {:<18} {}\r\n", entry.usage, entry.description));
        }

        text.push_str("Type help <command> for details.\r\n");

        text
    }

    pub(crate) fn get_item(&mut self, session_id: SessionId, requested_name: &str) -> GameResult {
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

    pub(crate) fn drop_item(&mut self, session_id: SessionId, requested_name: &str) -> GameResult {
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

    pub(crate) fn show_inventory(&mut self, session_id: SessionId) -> GameResult {
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
    ) -> GameResult {
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

    pub(crate) fn give_item(
        &mut self,
        giver_id: SessionId,
        requested_item: &str,
        requested_recipient: &str,
    ) -> GameResult {
        let (giver_name, room) = self
            .sessions
            .get(&giver_id)
            .and_then(ConnectedSession::player)
            .map(|player| (player.name().to_owned(), player.room()))
            .ok_or("choose a name first")?;

        let normalized_recipient = requested_recipient.trim().to_lowercase();

        if giver_name.to_lowercase() == normalized_recipient {
            return Err("you cannot give an item to yourself".into());
        }

        let recipient_id = self
            .sessions
            .iter()
            .find_map(|(&session_id, session)| {
                let player = session.player()?;

                (session_id != giver_id
                    && player.room() == room
                    && player.name().to_lowercase() == normalized_recipient)
                    .then_some(session_id)
            })
            .ok_or("that player is not here")?;

        let recipient_name = self
            .sessions
            .get(&recipient_id)
            .and_then(ConnectedSession::player)
            .expect("recipient was checked above")
            .name()
            .to_owned();

        let item = self
            .sessions
            .get_mut(&giver_id)
            .and_then(ConnectedSession::player_mut)
            .expect("giver was checked above")
            .take_item(requested_item)
            .ok_or("you are not carrying that item")?;

        let item_name = item.name().to_owned();

        self.sessions
            .get_mut(&recipient_id)
            .and_then(ConnectedSession::player_mut)
            .expect("recipient was checked above")
            .give_item(item);

        self.send_text(
            giver_id,
            format!("You give {item_name} to {recipient_name}.\r\n"),
        );

        self.send_text(
            recipient_id,
            format!("{giver_name} gives you {item_name}.\r\n"),
        );

        Ok(())
    }

    fn apply_hook_effects(&mut self, actor_id: SessionId, effects: Vec<Effect>) {
        for effect in effects {
            match effect.into_kind() {
                EffectKind::Reply(text) => {
                    self.send_text(actor_id, text);
                }
                EffectKind::Broadcast { room, text } => {
                    self.send_room(room, None, text);
                }
            }
        }
    }
}
