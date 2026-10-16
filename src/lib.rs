//! FerrumMUD is an asynchronous, embeddable engine for line-oriented text games.
//!
//! The crate provides a world model, a command registry, event hooks, and a Tokio
//! actor that serializes access to connected players and game state. Use
//! [`load_world`] to load the JSON format used by the bundled server, or build a
//! [`World`] programmatically.
//!
//! # Example
//!
//! ```
//! use ferrum_rs::{Direction, World};
//!
//! let mut world = World::new("Crossroads", "Four roads meet here.");
//! let crossroads = world.start_room();
//! let inn = world.add_room("Inn", "A welcoming fire burns here.")?;
//! world.connect_two_way(crossroads, Direction::North, inn)?;
//!
//! # Ok::<(), ferrum_rs::WorldError>(())
//! ```

#![warn(missing_docs)]

/// Command traits, execution context, and command registry.
pub mod command;
/// Built-in player commands.
pub mod commands;
/// Errors shared by game commands and hooks.
pub mod error;
/// Event hooks and the effects they can produce.
pub mod hooks;
/// Item identifiers and item metadata.
pub mod item;
/// Player state and player-name validation.
pub mod player;
/// Server actor, handles, session identifiers, and outbound events.
pub mod server;
/// TCP client-session processing.
pub mod session;
/// Rooms, exits, directions, and in-world item storage.
pub mod world;
/// JSON world loading and validation.
pub mod world_loader;

pub use command::{Command, CommandContext, CommandHelp, CommandRegistry};

pub use player::{Player, PlayerNameError};

pub use server::{ServerActor, ServerHandle};

pub use session::process_client;

pub use world::{Direction, RoomId, World, WorldError};

pub use world_loader::{WorldLoadError, load_world, parse_world};

pub use item::{Item, ItemId};

pub use hooks::{Effect, GameEvent, Hook, HookRegistry};

pub use error::{GameError, GameResult};
