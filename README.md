<p align="center">
  <img src="https://raw.githubusercontent.com/teyoku/ferrum-rs/main/media/logo.png" alt="FerrumMUD logo" height="250">
</p>

# FerrumMUD

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Built with Tokio](https://img.shields.io/badge/built%20with-tokio-blue.svg)](https://tokio.rs)
[![docs.rs](https://img.shields.io/docsrs/ferrum-rs)](https://docs.rs/ferrum-rs)
[![crates.io](https://img.shields.io/crates/v/ferrum-rs)](https://crates.io/crates/ferrum-rs)
[![GitHub Stars](https://img.shields.io/github/stars/teyoku/ferrum-rs?logo=github)](https://github.com/teyoku/ferrum-rs/stargazers)
[![Last Commit](https://img.shields.io/github/last-commit/teyoku/ferrum-rs?logo=github)](https://github.com/teyoku/ferrum-rs/commits)
[![Open Issues](https://img.shields.io/github/issues/teyoku/ferrum-rs?logo=github)](https://github.com/teyoku/ferrum-rs/issues)

FerrumMUD is a small, asynchronous MUD engine written in Rust. It accepts plain-text TCP connections, runs all game-state mutations through a single server actor, and loads rooms, exits, and items from a JSON file.

The project is intentionally compact: it can be used as a playable demo, a reference implementation for a Tokio actor, or a foundation for a larger text-based game.

## Features

- Multiple concurrent TCP sessions powered by Tokio
- JSON-defined rooms, directional exits, and items
- Player naming, movement, room chat, inventory, item examination, and trading
- Extensible command registry with aliases and generated in-game help
- Before/after event hooks that can reject actions or emit messages
- Case-insensitive command, player-name, and item-name matching
- Unit and asynchronous integration tests

## Requirements

- Rust 1.85 or newer (the crate uses the Rust 2024 edition)
- A TCP client such as `telnet`, `nc`, or MUDlet

## Installation

Install the standalone server from crates.io:

```console
cargo install ferrum-rs
```

Or add the engine to another Rust project:

```console
cargo add ferrum-rs
```

The example world from `data/world.json` is embedded in the executable and is
used whenever `WORLD_FILE` is unset.

## Quick start

Clone the repository and start the server:

```console
cargo run
```

The default address is `127.0.0.1:4000`. Connect from another terminal:

```console
nc 127.0.0.1 4000
```

Then choose a name and explore:

```text
name Alice
look
north
say Hello!
help
```

## Configuration

FerrumMUD reads a `.env` file when present and supports these environment variables:

| Variable     | Default           | Purpose                                    |
| ------------ | ----------------- | ------------------------------------------ |
| `HOST`       | `127.0.0.1`       | Interface on which the TCP listener binds  |
| `PORT`       | `4000`            | TCP port on which the server listens       |
| `WORLD_FILE` | bundled example   | Path to a custom JSON world definition     |
| `RUST_LOG`   | unset             | Log filter consumed by `pretty_env_logger` |

For example:

```console
HOST=0.0.0.0 PORT=5000 WORLD_FILE=data/world.json cargo run
```

The protocol is line-oriented UTF-8 text. Server output uses CRLF line endings so traditional Telnet clients render it correctly. Input lines are limited to 4 KiB.

## Player commands

| Command                | Aliases                                                                      | Description                                 |
| ---------------------- | ---------------------------------------------------------------------------- | ------------------------------------------- |
| `name <player>`        | —                                                                            | Choose or change a player name              |
| `help [command]`       | `?`                                                                          | List commands or show command-specific help |
| `who`                  | —                                                                            | List connected players                      |
| `look`                 | `l`                                                                          | Describe the current room                   |
| `go <direction>`       | `north`, `n`, `south`, `s`, `east`, `e`, `west`, `w`, `up`, `u`, `down`, `d` | Move through an exit                        |
| `say <message>`        | `'`                                                                          | Speak to players in the same room           |
| `get <item>`           | `take`                                                                       | Pick up an item                             |
| `drop <item>`          | —                                                                            | Drop a carried item                         |
| `inventory`            | `inv`, `i`                                                                   | List carried items                          |
| `examine <item>`       | `exam`, `x`                                                                  | Examine a nearby or carried item            |
| `give <item> <player>` | —                                                                            | Give an item to a player in the same room   |
| `quit`                 | `q`, `exit`                                                                  | Close the session                           |

Only `name`, `help`, and `quit` are available before a player chooses a name. Names must contain 3–16 Unicode alphabetic characters and must be unique among connected players.

## World files

A world is a JSON object containing a start-room key, a non-empty list of rooms, and an optional list of exits:

```json
{
  "start": "square",
  "rooms": [
    {
      "key": "square",
      "name": "Village Square",
      "description": "A stone well stands here.",
      "items": [
        {
          "name": "brass key",
          "description": "A small key polished by many hands."
        }
      ]
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
```

Room keys must be unique and non-empty. Directions accept their full names or one-letter forms. A bidirectional exit automatically creates the reverse exit using the opposite direction. See [`data/world.json`](data/world.json) for the bundled example.

## Using FerrumMUD as a library

The crate exposes the world model, command system, hook system, server actor, and world loader. A server can be assembled programmatically:

```rust
use ferrum_rs::{Direction, ServerActor, World};

#[tokio::main]
async fn main() {
    let mut world = World::new("Crossroads", "Four roads meet here.");
    let crossroads = world.start_room();
    let inn = world
        .add_room("Roadside Inn", "Warm light spills from the windows.")
        .unwrap();

    world
        .connect_two_way(crossroads, Direction::North, inn)
        .unwrap();

    let _server = ServerActor::spawn_with_world(world);
}
```

Implement [`Command`](src/command.rs) to add game commands and [`Hook`](src/hooks.rs) to observe, reject, or augment movement and speech events. Custom registries can be passed to `ServerActor::spawn_with_registry_and_world` or `ServerActor::spawn_with_registry_hooks_and_world`.

Generate and open the API documentation with:

```console
cargo doc --open
```

## Architecture

Each network connection is handled by its own asynchronous session task. Sessions exchange messages with one `ServerActor` through bounded Tokio channels. The actor owns all connected players and the world, so state changes are serialized without shared mutable locks.

```text
TCP clients -> session tasks -> ServerHandle -> ServerActor -> World
                    ^                            |
                    +------ SessionEvent --------+
```

The actor resolves input through the command registry. Commands operate through a restricted `CommandContext`, while hooks run immediately before and after supported game events.

## Development

Run the standard checks before submitting a change:

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --no-deps
```

Before publishing a release, update the version and [`CHANGELOG.md`](CHANGELOG.md),
commit all intended files, and inspect the package with:

```console
cargo publish --dry-run
```

## License

FerrumMUD is dual-licensed under either of the following, at your option:

- [Apache License 2.0](LICENSE-APACHE.md)
- [MIT License](LICENSE-MIT.md)
