use std::{fmt::Debug, sync::Arc};

use crate::{
    error::GameResult,
    world::{Direction, RoomId},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    Move {
        player_name: String,
        from: RoomId,
        to: RoomId,
        direction: Direction,
    },

    Say {
        player_name: String,
        room: RoomId,
        message: String,
    },
}

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
    pub fn reply(text: impl Into<String>) -> Self {
        Self {
            kind: EffectKind::Reply(with_newline(text)),
        }
    }

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
pub trait Hook: Debug + Send + Sync {
    fn before(&self, _event: &GameEvent) -> GameResult<Vec<Effect>> {
        Ok(Vec::new())
    }

    fn after(&self, _event: &GameEvent) -> Vec<Effect> {
        Vec::new()
    }
}

#[derive(Debug, Default)]
pub struct HookRegistry {
    hooks: Vec<Arc<dyn Hook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

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
