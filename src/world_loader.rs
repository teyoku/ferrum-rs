use serde::Deserialize;

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
