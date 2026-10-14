use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(u64);

impl RoomId {
    pub fn new(value: u64) -> Self {
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
        match input.trim().to_lowercase().as_str() {
            "north" | "n" => Some(Direction::North),
            "south" | "s" => Some(Direction::South),
            "east" | "e" => Some(Direction::East),
            "west" | "w" => Some(Direction::West),
            "up" | "u" => Some(Direction::Up),
            "down" | "d" => Some(Direction::Down),
            _ => None,
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
}

#[derive(Debug)]
struct Room {
    name: String,
    description: String,
    exits: HashMap<Direction, RoomId>,
}

#[derive(Debug)]
pub struct World {
    rooms: HashMap<RoomId, Room>,
    start_room: RoomId,
}

impl World {
    pub fn demo() -> Self {
        let square = RoomId(1);
        let tavern = RoomId(2);

        let rooms = HashMap::from([
            (
                square,
                Room {
                    name: "Town Square".to_owned(),
                    description: "A weathered fountain stands in the middle of the square."
                        .to_owned(),
                    exits: HashMap::from([(Direction::North, tavern)]),
                },
            ),
            (
                tavern,
                Room {
                    name: "Rusty Tankard".to_owned(),
                    description: "The tavern is warm and smells of wood smoke.".to_owned(),
                    exits: HashMap::from([(Direction::South, square)]),
                },
            ),
        ]);

        Self {
            rooms,
            start_room: square,
        }
    }

    pub fn start_room(&self) -> RoomId {
        self.start_room
    }

    pub fn destination(&self, room: RoomId, direction: Direction) -> Option<RoomId> {
        self.rooms.get(&room)?.exits.get(&direction).copied()
    }

    pub fn describe(&self, room: RoomId) -> Option<String> {
        let room = self.rooms.get(&room)?;

        let mut exits: Vec<_> = room
            .exits
            .keys()
            .map(|direction| direction.as_str())
            .collect();

        exits.sort_unstable();

        let exits = if exits.is_empty() {
            "none".to_owned()
        } else {
            exits.join(", ")
        };

        Some(format!(
            "{}\r\n{}\r\nExits: {}\r\n",
            room.name, room.description, exits
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_world_has_a_two_way_path() {
        let world = World::demo();
        let square = world.start_room();

        let tavern = world.destination(square, Direction::North).unwrap();

        assert_eq!(world.destination(tavern, Direction::South), Some(square));
    }
}
