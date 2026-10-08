use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldDefinition {
    pub version: u32,
    pub start_room: String,
    pub rooms: Vec<RoomDefinition>,

    #[serde(default)]
    pub connections: Vec<ConnectionDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomDefinition {
    pub key: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionDefinition {
    pub from: String,
    pub direction: String,
    pub to: String,

    #[serde(default)]
    pub bidirectional: bool,
}
