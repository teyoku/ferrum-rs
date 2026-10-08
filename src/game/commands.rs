use crate::game::{
    command::{Command, CommandContext, CommandResult},
    registry::CommandRegistry,
    room::Direction,
};

pub fn register_all(registry: &mut CommandRegistry) -> Result<(), String> {
    registry.register(Look)?;
    registry.register(Move)?;
    registry.register(Say)?;
    registry.register(Help)?;
    registry.register(Quit)?;

    registry.register(DirectionCommand::new("north", &["n"], Direction::North))?;
    registry.register(DirectionCommand::new("south", &["s"], Direction::South))?;
    registry.register(DirectionCommand::new("east", &["e"], Direction::East))?;
    registry.register(DirectionCommand::new("west", &["w"], Direction::West))?;
    registry.register(DirectionCommand::new("up", &["u"], Direction::Up))?;
    registry.register(DirectionCommand::new("down", &["d"], Direction::Down))?;

    Ok(())
}

struct Look;

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

pub struct Move;

impl Command for Move {
    fn name(&self) -> &'static str {
        "go"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["move"]
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> CommandResult {
        if ctx.args.is_empty() {
            return Err("What direction?".to_string());
        }

        let direction =
            Direction::parse(ctx.args).ok_or_else(|| format!("Unknown direction: {}", ctx.args))?;

        move_and_look(ctx, direction)
    }
}

struct DirectionCommand {
    name: &'static str,
    aliases: &'static [&'static str],
    direction: Direction,
}

impl DirectionCommand {
    const fn new(
        name: &'static str,
        aliases: &'static [&'static str],
        direction: Direction,
    ) -> Self {
        Self {
            name,
            aliases,
            direction,
        }
    }
}

impl Command for DirectionCommand {
    fn name(&self) -> &'static str {
        self.name
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.aliases
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> CommandResult {
        move_and_look(ctx, self.direction)
    }
}

fn move_and_look(ctx: &mut CommandContext<'_>, direction: Direction) -> CommandResult {
    ctx.world.move_player(ctx.player_id, direction)?;

    let description = ctx.world.describe_current_room(ctx.player_id)?;
    ctx.send(description)
}

struct Say;

impl Command for Say {
    fn name(&self) -> &'static str {
        "say"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> CommandResult {
        if ctx.args.is_empty() {
            return Err("What to say?".to_string());
        }

        let players = ctx.world.players();
        let current_player = players
            .get(&ctx.player_id)
            .ok_or_else(|| "Player does not exist".to_string())?;

        for (player_id, player) in players {
            if player.room() == current_player.room() {
                let (sender, verb) = if *player_id == ctx.player_id {
                    ("You", "say")
                } else {
                    (current_player.name(), "says")
                };

                let message = format!("{sender} {verb}: \"{}\"\n", ctx.args);
                let _ = ctx.world.send_to(*player_id, message);
            }
        }

        Ok(())
    }
}

struct Help;

impl Command for Help {
    fn name(&self) -> &'static str {
        "help"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["commands"]
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> CommandResult {
        let mut names = ctx.registry.names().collect::<Vec<_>>();
        names.sort_unstable();

        ctx.send(format!("Commands: {}\n", names.join(", ")))
    }
}

struct Quit;

impl Command for Quit {
    fn name(&self) -> &'static str {
        "quit"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["q", "exit"]
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> CommandResult {
        let _ = ctx.send("Goodbye!\n");
        ctx.world.disconnect(ctx.client_id);

        Ok(())
    }
}
