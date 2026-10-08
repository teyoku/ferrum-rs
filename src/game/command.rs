use crate::game::{event::ClientId, player::PlayerId, room::Direction, world::World};

pub type CommandResult = Result<(), String>;

pub struct CommandContext<'a> {
    pub world: &'a mut World,
    pub registry: &'a CommandRegistry,
    pub client_id: ClientId,
    pub player_id: PlayerId,
    pub args: &'a str
}

impl<'a> CommandContext<'a> {
    pub fn send(&self, message: impl Into<String>) -> CommandResult {
        self.world.send_to(self.player_id, message)
    }
}

pub trait Command: Send + Sync {
    fn name(&self) -> &'static str;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> CommandResult;
}