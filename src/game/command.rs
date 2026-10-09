use crate::game::{
    event::{ClientId, EngineEffect},
    player::PlayerId,
    registry::CommandRegistry,
    world::World,
};

pub type CommandResult = Result<(), String>;

pub struct CommandContext<'a> {
    pub world: &'a mut World,
    pub registry: &'a CommandRegistry,
    pub client_id: ClientId,
    pub player_id: PlayerId,
    pub args: &'a str,
    pub effects: &'a mut Vec<EngineEffect>,
}

impl<'a> CommandContext<'a> {
    pub fn send(&mut self, message: impl Into<String>) -> CommandResult {
        self.effects.push(EngineEffect::Send {
            session_id: self.client_id,
            message: message.into(),
        });
        Ok(())
    }
}

pub trait Command: Send + Sync {
    fn name(&self) -> &'static str;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> CommandResult;
}
