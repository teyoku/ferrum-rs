use std::{fmt::Debug, sync::Arc};

use crate::{
    error::GameResult,
    world::{Direction, RoomId},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveEvent {
    pub player_name: String,
    pub from: RoomId,
    pub to: RoomId,
    pub direction: Direction,
}

pub trait Hook: Debug + Send + Sync {
    fn before_move(&self, _event: &MoveEvent) -> GameResult {
        Ok(())
    }

    fn after_move(&self, _event: &MoveEvent) {}
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

    pub(crate) fn before_move(&self, event: &MoveEvent) -> GameResult {
        for hook in &self.hooks {
            hook.before_move(event)?;
        }

        Ok(())
    }

    pub(crate) fn after_move(&self, event: &MoveEvent) {
        for hook in &self.hooks {
            hook.after_move(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct ClosedGate;

    impl Hook for ClosedGate {
        fn before_move(&self, event: &MoveEvent) -> GameResult {
            Err(format!("the {} gate is closed", event.direction.as_str()).into())
        }
    }

    #[test]
    fn a_hook_can_cancel_movement() {
        let mut hooks = HookRegistry::new();
        hooks.register(ClosedGate);

        let event = MoveEvent {
            player_name: "Alex".to_owned(),
            from: RoomId::new(1),
            to: RoomId::new(2),
            direction: Direction::North,
        };

        assert_eq!(
            hooks.before_move(&event),
            Err("the north gate is closed".into())
        );
    }
}
