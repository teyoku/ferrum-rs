use crate::game::command::{Command, CommandContext, CommandResult};

pub struct Look;

impl Command for Look {
    fn name(&self) -> &'static str {
        "look"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["l"]
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> CommandResult {
        let description = ctx.world.describe_current_room(ctx.player_id)?;
        ctx.send(description)
    }
}