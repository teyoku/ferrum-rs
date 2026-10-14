use std::{error::Error, fmt, io};

use serde::Deserialize;

use crate::world::WorldError;

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
