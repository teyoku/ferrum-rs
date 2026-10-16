use std::{fmt::Debug, sync::Arc};

use crate::world::{Direction, RoomId};

pub struct MoveEvent {
    pub player_name: String,
    pub from: RoomId,
    pub to: RoomId,
    pub direction: Direction,
}

pub trait Hook: Debug + Send + Sync {
    fn before_move(&self, _event: &MoveEvent) -> Result<(), &'static str> {
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

    pub(crate) fn before_move(&self, event: &MoveEvent) -> Result<(), &'static str> {
        for hook in &self.hooks {
            hook.before_move(event)?;
        }

        Ok(())
    }

    pub(crate) fn after_move(&self, event: &MoveEvent) {
        for hook in &self.hooks {
            self.after_move(event);
        }
    }
}
