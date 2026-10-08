use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(u32);

impl RoomId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Direction {
    pub fn parse(input: &str) -> Option<Self> {
        match input.to_ascii_lowercase().as_str() {
            "n" | "north" => Some(Self::North),
            "s" | "south" => Some(Self::South),
            "e" | "east" => Some(Self::East),
            "w" | "west" => Some(Self::West),
            "u" | "up" => Some(Self::Up),
            "d" | "down" => Some(Self::Down),
            _ => None,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::South => "south",
            Self::East => "east",
            Self::West => "west",
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

pub struct Room {
    title: String,
    description: String,
    exits: HashMap<Direction, RoomId>,
}

impl Room {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            exits: HashMap::new(),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn exits(&self) -> &HashMap<Direction, RoomId> {
        &self.exits
    }

    pub fn add_exit(&mut self, direction: Direction, destination: RoomId) {
        self.exits.insert(direction, destination);
    }

    pub fn exit(&self, direction: Direction) -> Option<RoomId> {
        self.exits.get(&direction).copied()
    }
}
