use crate::item::Item;
use core::fmt;
use std::{collections::HashMap, error::Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(u64);

impl RoomId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    RoomNotFound(RoomId),
    RoomIdOverflow,
    EmptyRoomKey,
    DuplicateRoomKey(String),
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoomNotFound(room_id) => {
                write!(f, "room {room_id} does not exist")
            }
            Self::RoomIdOverflow => f.write_str("room identifier overflowed"),
            Self::EmptyRoomKey => f.write_str("room key must not be empty"),
            Self::DuplicateRoomKey(key) => write!(f, "duplicate room key: {key}"),
        }
    }
}

impl Error for WorldError {}

#[derive(Debug)]
struct Room {
    name: String,
    description: String,
    exits: HashMap<Direction, RoomId>,
    items: Vec<Item>,
}

#[derive(Debug)]
pub struct World {
    rooms: HashMap<RoomId, Room>,
    room_ids: HashMap<String, RoomId>,
    room_keys: HashMap<RoomId, String>,
    start_room: RoomId,
    next_room_id: u64,
}

impl World {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new_with_key("start", name, description)
            .expect("default start room key must be valid")
    }

    pub fn new_with_key(
        key: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, WorldError> {
        let key = key.into();
        let key = key.trim();

        if key.is_empty() {
            return Err(WorldError::EmptyRoomKey);
        }

        let key = key.to_owned();
        let start_room = RoomId::new(1);

        let room = Room {
            name: name.into(),
            description: description.into(),
            exits: HashMap::new(),
            items: Vec::new(),
        };

        Ok(Self {
            rooms: HashMap::from([(start_room, room)]),
            room_ids: HashMap::from([(key.clone(), start_room)]),
            room_keys: HashMap::from([(start_room, key)]),
            start_room,
            next_room_id: 2,
        })
    }

    pub(crate) fn demo() -> Self {
        let mut world = Self::new(
            "Town Square",
            "A weathered fountain stands in the middle of the square.",
        );

        let square = world.start_room();

        let tavern = world
            .add_room(
                "Rusty Tankard",
                "The tavern is warm and smells of wood smoke.",
            )
            .expect("demo world must fit into room identifiers");

        world
            .connect_two_way(square, Direction::North, tavern)
            .expect("demo rooms must exist");

        world
    }

    pub fn start_room(&self) -> RoomId {
        self.start_room
    }

    pub fn set_start_room(&mut self, room: RoomId) -> Result<(), WorldError> {
        self.require_room(room)?;

        self.start_room = room;

        Ok(())
    }

    pub fn room_id(&self, key: &str) -> Option<RoomId> {
        self.room_ids.get(key.trim()).copied()
    }

    pub fn room_key(&self, room: RoomId) -> Option<&str> {
        self.room_keys.get(&room).map(String::as_str)
    }

    pub fn add_room(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<RoomId, WorldError> {
        let key = format!("room-{}", self.next_room_id);

        self.add_room_with_key(key, name, description)
    }

    pub fn add_room_with_key(
        &mut self,
        key: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<RoomId, WorldError> {
        let key = key.into();
        let key = key.trim();

        if key.is_empty() {
            return Err(WorldError::EmptyRoomKey);
        }

        if self.room_ids.contains_key(key) {
            return Err(WorldError::DuplicateRoomKey(key.to_owned()));
        }

        let key = key.to_owned();
        let room_id = RoomId::new(self.next_room_id);

        self.next_room_id = self
            .next_room_id
            .checked_add(1)
            .ok_or(WorldError::RoomIdOverflow)?;

        self.rooms.insert(
            room_id,
            Room {
                name: name.into(),
                description: description.into(),
                exits: HashMap::new(),
                items: Vec::new(),
            },
        );

        self.room_ids.insert(key.clone(), room_id);
        self.room_keys.insert(room_id, key);

        Ok(room_id)
    }

    pub fn connect(
        &mut self,
        from: RoomId,
        direction: Direction,
        to: RoomId,
    ) -> Result<(), WorldError> {
        self.require_room(to)?;

        let room = self
            .rooms
            .get_mut(&from)
            .ok_or(WorldError::RoomNotFound(from))?;

        room.exits.insert(direction, to);

        Ok(())
    }

    pub fn connect_two_way(
        &mut self,
        first: RoomId,
        direction: Direction,
        second: RoomId,
    ) -> Result<(), WorldError> {
        self.require_room(first)?;
        self.require_room(second)?;

        self.rooms
            .get_mut(&first)
            .expect("room was checked above")
            .exits
            .insert(direction, second);

        self.rooms
            .get_mut(&second)
            .expect("room was checked above")
            .exits
            .insert(direction.opposite(), first);

        Ok(())
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

        let items = if room.items.is_empty() {
            "none".to_owned()
        } else {
            room.items
                .iter()
                .map(Item::name)
                .collect::<Vec<_>>()
                .join(", ")
        };

        Some(format!(
            "{}\r\n{}\r\nExits: {}\r\nItems here: {}\r\n",
            room.name, room.description, exits, items
        ))
    }

    fn require_room(&self, room: RoomId) -> Result<(), WorldError> {
        self.rooms
            .contains_key(&room)
            .then_some(())
            .ok_or(WorldError::RoomNotFound(room))
    }

    pub fn add_item(&mut self, room: RoomId, item: Item) -> Result<(), WorldError> {
        let room = self
            .rooms
            .get_mut(&room)
            .ok_or(WorldError::RoomNotFound(room))?;

        room.items.push(item);

        Ok(())
    }

    pub fn take_item(&mut self, room: RoomId, name: &str) -> Option<Item> {
        let room = self.rooms.get_mut(&room)?;

        let index = room.items.iter().position(|item| item.matches_name(name))?;

        Some(room.items.remove(index))
    }

    pub fn find_item(&self, room: RoomId, name: &str) -> Option<&Item> {
        self.rooms
            .get(&room)?
            .items
            .iter()
            .find(|item| item.matches_name(name))
    }
}

#[cfg(test)]
mod tests {
    use crate::world::WorldError::RoomNotFound;

    use super::*;

    #[test]
    fn demo_world_has_a_two_way_path() {
        let world = World::demo();
        let square = world.start_room();

        let tavern = world.destination(square, Direction::North).unwrap();

        assert_eq!(world.destination(tavern, Direction::South), Some(square));
    }

    #[test]
    fn builds_a_custom_world() {
        let mut world = World::new("Dock", "Cold waves strike the old pier.");

        let dock = world.start_room();

        let warehouse = world
            .add_room("Warehouse", "Dusty crates fill the room.")
            .unwrap();

        world
            .connect_two_way(dock, Direction::East, warehouse)
            .unwrap();

        assert_eq!(world.destination(dock, Direction::East), Some(warehouse));

        assert_eq!(world.destination(warehouse, Direction::West), Some(dock));
    }

    #[test]
    fn refuses_to_connect_a_missing_room() {
        let mut world = World::new("Dock", "Cold waves strike the old pier.");

        let dock = world.start_room();
        let missing = RoomId::new(999);

        assert_eq!(
            world.connect(dock, Direction::North, missing),
            Err(RoomNotFound(missing))
        );
    }

    #[test]
    fn resolves_rooms_by_stable_key() {
        let mut world =
            World::new_with_key("dock", "Dock", "Cold waves strike the old pier.").unwrap();

        let warehouse = world
            .add_room_with_key("warehouse", "Warehouse", "Dusty crates fill the room.")
            .unwrap();

        assert_eq!(world.room_id("dock"), Some(world.start_room()));

        assert_eq!(world.room_id("warehouse"), Some(warehouse));

        assert_eq!(world.room_key(warehouse), Some("warehouse"));
    }
}
