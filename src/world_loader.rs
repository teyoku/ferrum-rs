use std::io;

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
