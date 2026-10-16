use std::{fmt::Debug, sync::Arc};

use crate::{
    error::GameResult,
    world::{Direction, RoomId},
};

/// A game action that hooks can inspect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    /// A player is moving between rooms.
    Move {
        /// Display name of the moving player.
        player_name: String,
        /// Room occupied before the move.
        from: RoomId,
        /// Destination room.
        to: RoomId,
        /// Direction selected by the player.
        direction: Direction,
    },

    /// A player is speaking in a room.
    Say {
        /// Display name of the speaking player.
        player_name: String,
        /// Room in which the message is spoken.
        room: RoomId,
        /// Message supplied by the player.
        message: String,
    },
}

/// A message-producing side effect returned by a [`Hook`].
///
/// Effects are applied in hook registration order. Text is normalized to end
/// in a CRLF sequence when the effect is created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    kind: EffectKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EffectKind {
    Reply(String),

    Broadcast { room: RoomId, text: String },
}

impl Effect {
    /// Creates an effect that replies to the player who triggered the event.
    pub fn reply(text: impl Into<String>) -> Self {
        Self {
            kind: EffectKind::Reply(with_newline(text)),
        }
    }

    /// Creates an effect that broadcasts text to every player in `room`.
    pub fn broadcast(room: RoomId, text: impl Into<String>) -> Self {
        Self {
            kind: EffectKind::Broadcast {
                room,
                text: with_newline(text),
            },
        }
    }

    pub(crate) fn into_kind(self) -> EffectKind {
        self.kind
    }
}

fn with_newline(text: impl Into<String>) -> String {
    let mut text = text.into();

    if !text.ends_with("\r\n") {
        text.push_str("\r\n");
    }

    text
}
/// Observes game events and optionally produces additional effects.
///
/// Returning an error from [`Hook::before`] rejects the underlying action.
/// After hooks cannot reject an action because it has already taken place.
pub trait Hook: Debug + Send + Sync {
    /// Runs before an event is committed.
    fn before(&self, _event: &GameEvent) -> GameResult<Vec<Effect>> {
        Ok(Vec::new())
    }

    /// Runs after an event has been committed.
    fn after(&self, _event: &GameEvent) -> Vec<Effect> {
        Vec::new()
    }
}

/// An ordered collection of event hooks.
#[derive(Debug, Default)]
pub struct HookRegistry {
    hooks: Vec<Arc<dyn Hook>>,
}

impl HookRegistry {
    /// Creates an empty hook registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a hook to the registry.
    pub fn register<H>(&mut self, hook: H)
    where
        H: Hook + 'static,
    {
        self.hooks.push(Arc::new(hook));
    }

    pub(crate) fn before(&self, event: &GameEvent) -> GameResult<Vec<Effect>> {
        let mut effects = Vec::new();

        for hook in &self.hooks {
            effects.extend(hook.before(event)?);
        }

        Ok(effects)
    }

    pub(crate) fn after(&self, event: &GameEvent) -> Vec<Effect> {
        self.hooks
            .iter()
            .flat_map(|hook| hook.after(event))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct ClosedGate;

    impl Hook for ClosedGate {
        fn before(&self, event: &GameEvent) -> GameResult<Vec<Effect>> {
            let GameEvent::Move { direction, .. } = event else {
                return Ok(Vec::new());
            };

            Err(format!("the {} gate is closed", direction.as_str()).into())
        }
    }

    #[test]
    fn a_hook_can_cancel_movement() {
        let mut hooks = HookRegistry::new();
        hooks.register(ClosedGate);

        let event = GameEvent::Move {
            player_name: "Alex".to_owned(),
            from: RoomId::new(1),
            to: RoomId::new(2),
            direction: Direction::North,
        };

        assert_eq!(hooks.before(&event), Err("the north gate is closed".into()));
    }

    #[test]
    fn effect_adds_a_network_newline() {
        assert_eq!(
            Effect::reply("A whisper reaches you.").into_kind(),
            EffectKind::Reply("A whisper reaches you.\r\n".to_owned())
        );
    }
}
