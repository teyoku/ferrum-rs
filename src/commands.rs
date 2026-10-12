use crate::command::{Command, CommandContext, CommandRegistry, CommandResult};

pub fn register_builtins(registry: &mut CommandRegistry) {
    registry.register(NameCommand);
    registry.register(WhoCommand);
    registry.register(SayCommand);
    registry.register(QuitCommand);
}

#[derive(Debug)]
pub struct WhoCommand;

impl Command for WhoCommand {
    fn name(&self) -> &'static str {
        "who"
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
pub struct QuitCommand;

impl Command for QuitCommand {
    fn name(&self) -> &'static str {
        "quit"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["q", "exit"]
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        context.close();

        Ok(())
    }
}

#[derive(Debug)]
pub struct NameCommand;

impl Command for NameCommand {
    fn name(&self) -> &'static str {
        "name"
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult {
        let requested_name = context.args.trim().to_owned();

        match context.set_name(&requested_name) {
            Ok(()) => context.reply(format!(
                "You are now known as \
                 {requested_name}.\r\n"
            )),

            Err(err) => context.reply(format!("Cannot use that name: {err}.\r\n")),
        }

        Ok(())
    }
}
