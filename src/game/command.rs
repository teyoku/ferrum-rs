use crate::game::room::Direction;

pub enum Command {
    Look,
    Move(Direction),
    Say(String),
    Help,
    Quit,
}

impl Command {
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut words = input.split_whitespace();
        let Some(verb) = words.next() else {
            return Err("Please write a command".to_string());
        };

        let verb = verb.to_ascii_lowercase();

        match verb.as_str() {
            "look" | "l" => Ok(Command::Look),

            "go" | "move" => {
                let Some(arg) = words.next() else {
                    return Err("What direction".to_string());
                };

                Direction::parse(arg)
                    .map(Command::Move)
                    .ok_or_else(|| format!("Unknown direction: {arg}"))
            }

            "north" | "n" => Ok(Command::Move(Direction::North)),
            "south" | "s" => Ok(Command::Move(Direction::South)),
            "east" | "e" => Ok(Command::Move(Direction::East)),
            "west" | "w" => Ok(Command::Move(Direction::West)),
            "up" | "u" => Ok(Command::Move(Direction::Up)),
            "down" | "d" => Ok(Command::Move(Direction::Down)),

            "say" => {
                let message = words.collect::<Vec<_>>().join(" ");
                if message.is_empty() {
                    Err("What to say?".to_string())
                } else {
                    Ok(Command::Say(message))
                }
            }

            "help" => Ok(Command::Help),
            "quit" => Ok(Command::Quit),

            _ => Err(format!("Unknown command: {verb}")),
        }
    }
}
