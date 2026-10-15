use crate::command::{Command, CommandContext, CommandRegistry, CommandResult};

pub fn register_builtins(registry: &mut CommandRegistry) {
    registry.register(NameCommand);
    registry.register(WhoCommand);
    registry.register(SayCommand);
    registry.register(LookCommand);
    registry.register(GoCommand);
    registry.register(HelpCommand);
    registry.register(GetCommand);
    registry.register(DropCommand);
    registry.register(InventoryCommand);
    registry.register(QuitCommand);
}

#[derive(Debug)]
pub struct NameCommand;

impl Command for NameCommand {
    fn name(&self) -> &'static str {
        "name"
    }

    fn allowed_before_login(&self) -> bool {
        true
    }

    fn usage(&self) -> &'static str {
        "name <player>"
    }

    fn description(&self) -> &'static str {
        "Choose or change your player name."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        let requested_name = context.args.trim().to_owned();

        match context.set_name(&requested_name) {
            Ok(()) => {
                context.reply(format!("You are now known as {requested_name}.\r\n"));

                context.look()?;
            }

            Err(err) => context.reply(format!("Cannot use that name: {err}.\r\n")),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct WhoCommand;

impl Command for WhoCommand {
    fn name(&self) -> &'static str {
        "who"
    }

    fn description(&self) -> &'static str {
        "List connected players."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        let names = context.online_names();
        let count = names.len();
        let list = names.join("\r\n");

        context.reply(format!("Players online: ({count})\r\n{list}\r\n"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct SayCommand;

impl Command for SayCommand {
    fn name(&self) -> &'static str {
        "say"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["'"]
    }

    fn usage(&self) -> &'static str {
        "say <message>"
    }

    fn description(&self) -> &'static str {
        "Speak to players in the same room."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        let message = context.args.trim().to_owned();

        if message.is_empty() {
            context.reply("What do you want to say?\r\n");
        } else {
            context.say(&message)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct LookCommand;

impl Command for LookCommand {
    fn name(&self) -> &'static str {
        "look"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["l"]
    }

    fn description(&self) -> &'static str {
        "Look at your current room."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        context.look()
    }
}

#[derive(Debug)]
pub struct GoCommand;

impl Command for GoCommand {
    fn name(&self) -> &'static str {
        "go"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &[
            "north", "n", "south", "s", "east", "e", "west", "w", "up", "u", "down", "d",
        ]
    }

    fn usage(&self) -> &'static str {
        "go <direction>"
    }

    fn description(&self) -> &'static str {
        "Move through an exit."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        let direction = if context.invoked_as == self.name() {
            context.args.trim()
        } else {
            context.invoked_as
        };

        if direction.is_empty() {
            context.reply("Go where?\r\n");
            return Ok(());
        }

        context.go(direction)
    }
}

#[derive(Debug)]
pub struct HelpCommand;

impl Command for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["?"]
    }

    fn allowed_before_login(&self) -> bool {
        true
    }

    fn description(&self) -> &'static str {
        "Show available commands."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        let text = context.help_text();

        context.reply(text);

        Ok(())
    }
}

#[derive(Debug)]
pub struct GetCommand;

impl Command for GetCommand {
    fn name(&self) -> &'static str {
        "get"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["take"]
    }

    fn usage(&self) -> &'static str {
        "get <item>"
    }

    fn description(&self) -> &'static str {
        "Pick up an item from the room."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        let name = context.args.trim().to_owned();

        if name.is_empty() {
            context.reply("Get what?\r\n");
            return Ok(());
        }

        context.get_item(&name)
    }
}

#[derive(Debug)]
pub struct DropCommand;

impl Command for DropCommand {
    fn name(&self) -> &'static str {
        "drop"
    }

    fn usage(&self) -> &'static str {
        "drop <item>"
    }

    fn description(&self) -> &'static str {
        "Drop an item into the room."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        let name = context.args.trim().to_owned();

        if name.is_empty() {
            context.reply("Drop what?\r\n");
            return Ok(());
        }

        context.drop_item(&name)
    }
}

#[derive(Debug)]
pub struct InventoryCommand;

impl Command for InventoryCommand {
    fn name(&self) -> &'static str {
        "inventory"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["inv", "i"]
    }

    fn description(&self) -> &'static str {
        "Show the items you are carrying."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        context.show_inventory()
    }
}

#[derive(Debug)]
pub struct QuitCommand;

impl Command for QuitCommand {
    fn name(&self) -> &'static str {
        "quit"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["q", "exit"]
    }

    fn allowed_before_login(&self) -> bool {
        true
    }

    fn description(&self) -> &'static str {
        "Disconnect from the game."
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        context.close();

        Ok(())
    }
}
