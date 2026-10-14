use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt, fs, io,
    path::Path,
};

use serde::Deserialize;

use crate::{
    RoomId,
    world::{Direction, World, WorldError},
};

#[derive(Debug, Deserialize)]
struct WorldDefinition {
    start: String,
    rooms: Vec<RoomDefinition>,

    #[serde(default)]
    exits: Vec<ExitDefinition>,
}

#[derive(Debug, Deserialize)]
struct RoomDefinition {
    key: String,
    name: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct ExitDefinition {
    from: String,
    direction: String,
    to: String,

    #[serde(default)]
    bidirectional: bool,
}

#[derive(Debug)]
pub enum WorldLoadError {
    Io(io::Error),
    Json(serde_json::Error),
    EmptyWorld,
    DuplicateRoomKey(String),
    StartRoomNotFound(String),
    RoomKeyNotFound(String),
    InvalidDirection(String),
    World(WorldError),
}

impl fmt::Display for WorldLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => {
                write!(f, "cannot read world file: {error}")
            }

            Self::Json(error) => {
                write!(f, "invalid world JSON: {error}")
            }

            Self::EmptyWorld => f.write_str("world must contain at least one room"),

            Self::DuplicateRoomKey(key) => {
                write!(f, "duplicate room key: {key}")
            }

            Self::StartRoomNotFound(key) => {
                write!(f, "start room key does not exist: {key}")
            }

            Self::RoomKeyNotFound(key) => {
                write!(f, "exit references unknown room key: {key}")
            }

            Self::InvalidDirection(direction) => {
                write!(f, "unknown exit direction: {direction}")
            }

            Self::World(error) => error.fmt(f),
        }
    }
}

impl Error for WorldLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::World(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for WorldLoadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for WorldLoadError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<WorldError> for WorldLoadError {
    fn from(error: WorldError) -> Self {
        Self::World(error)
    }
}

pub fn load_world(path: impl AsRef<Path>) -> Result<World, WorldLoadError> {
    let json = fs::read_to_string(path)?;

    parse_world(&json)
}

pub fn parse_world(json: &str) -> Result<World, WorldLoadError> {
    let definition = serde_json::from_str(json)?;

    build_world(definition)
}

fn build_world(definition: WorldDefinition) -> Result<World, WorldLoadError> {
    if definition.rooms.is_empty() {
        return Err(WorldLoadError::EmptyWorld);
    }

    validate_room_keys(&definition.rooms)?;

    let start = definition
        .rooms
        .iter()
        .find(|room| room.key == definition.start)
        .ok_or_else(|| WorldLoadError::StartRoomNotFound(definition.start.clone()))?;

    let mut world = World::new(&start.name, &start.description);

    let mut room_ids = HashMap::from([(start.key.as_str(), world.start_room())]);

    for room in &definition.rooms {
        if room.key == start.key {
            continue;
        }

        let room_id = world.add_room(&room.name, &room.description)?;

        room_ids.insert(room.key.as_str(), room_id);
    }

    for exit in &definition.exits {
        let from = find_room(&room_ids, &exit.from)?;
        let to = find_room(&room_ids, &exit.to)?;
        let direction = Direction::parse(&exit.direction)
            .ok_or_else(|| WorldLoadError::InvalidDirection(exit.direction.clone()))?;

        if exit.bidirectional {
            world.connect_two_way(from, direction, to)?;
        } else {
            world.connect(from, direction, to)?;
        }
    }

    Ok(world)
}

fn validate_room_keys(rooms: &[RoomDefinition]) -> Result<(), WorldLoadError> {
    let mut keys = HashSet::new();

    for room in rooms {
        if !keys.insert(room.key.as_str()) {
            return Err(WorldLoadError::DuplicateRoomKey(room.key.clone()));
        }
    }

    Ok(())
}

fn find_room(rooms: &HashMap<&str, RoomId>, key: &str) -> Result<RoomId, WorldLoadError> {
    rooms
        .get(key)
        .copied()
        .ok_or_else(|| WorldLoadError::RoomKeyNotFound(key.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_rooms_and_bidirectional_exits() {
        let world = parse_world(
            r#"
            {
                "start": "square",
                "rooms": [
                    {
                        "key": "square",
                        "name": "Village Square",
                        "description": "A stone well stands here."
                    },
                    {
                        "key": "tavern",
                        "name": "Old Tavern",
                        "description": "A fire crackles in the hearth."
                    }
                ],
                "exits": [
                    {
                        "from": "square",
                        "direction": "north",
                        "to": "tavern",
                        "bidirectional": true
                    }
                ]
            }
            "#,
        )
        .unwrap();

        let square = world.start_room();

        let tavern = world.destination(square, Direction::North).unwrap();

        assert_eq!(world.destination(tavern, Direction::South), Some(square));
    }

    #[test]
    fn rejects_duplicate_room_keys() {
        let error = parse_world(
            r#"
            {
                "start": "square",
                "rooms": [
                    {
                        "key": "square",
                        "name": "One",
                        "description": ""
                    },
                    {
                        "key": "square",
                        "name": "Two",
                        "description": ""
                    }
                ]
            }
            "#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            WorldLoadError::DuplicateRoomKey(key)
                if key == "square"
        ));
    }

    #[test]
    fn rejects_unknown_exit_room() {
        let error = parse_world(
            r#"
            {
                "start": "square",
                "rooms": [
                    {
                        "key": "square",
                        "name": "Square",
                        "description": ""
                    }
                ],
                "exits": [
                    {
                        "from": "square",
                        "direction": "north",
                        "to": "missing"
                    }
                ]
            }
            "#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            WorldLoadError::RoomKeyNotFound(key)
                if key == "missing"
        ));
    }
}
