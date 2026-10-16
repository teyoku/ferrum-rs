# AGENTS.md

This file contains repository-specific guidance for automated coding agents and other contributors working on FerrumMUD.

## Project overview

FerrumMUD is a Rust 2024, Tokio-based text MUD. The binary loads a JSON world, listens for line-oriented TCP connections, and delegates all mutable game state to a single server actor.

## Repository map

- `src/main.rs`: binary entry point and TCP listener
- `src/lib.rs`: public crate surface and re-exports
- `src/session.rs`: per-connection input/output loop
- `src/server.rs`: actor lifecycle, session state, and message transport
- `src/server/actions.rs`: game actions performed by the server actor
- `src/command.rs`: command trait, context, invocation, and registry
- `src/commands.rs`: built-in commands
- `src/hooks.rs`: game events, effects, and hook registry
- `src/world.rs`: rooms, directions, exits, and room inventory
- `src/world_loader.rs`: JSON parsing, validation, and world construction
- `src/player.rs`: player identity, location, and inventory
- `src/item.rs`: item identity and metadata
- `src/server/tests.rs`: asynchronous server integration tests
- `data/world.json`: bundled example world

## Working conventions

- Preserve the actor boundary: mutate sessions and world state only inside `ServerActor`.
- Keep network handling asynchronous and avoid blocking operations in Tokio tasks.
- Keep player-facing protocol output terminated with `\r\n`.
- Route command behavior through `CommandContext`; do not expose the complete server actor to external commands.
- Keep lookups case-insensitive where existing behavior is case-insensitive.
- Use checked arithmetic for generated identifiers.
- Prefer focused comments that explain invariants, ordering, or intent. Do not narrate straightforward Rust statements.
- Write all documentation, comments, identifiers, and player-facing text in English.
- Add Rustdoc comments to new public modules, types, variants, fields, constants, and methods.
- Avoid adding dependencies unless the standard library or an existing dependency cannot reasonably solve the problem.

## World-format rules

- A world must contain at least one room.
- Room keys are stable external identifiers and must be unique and non-empty.
- The `start` value must name an existing room key.
- Exit endpoints must name existing room keys.
- Supported directions are north, south, east, west, up, and down, including their one-letter abbreviations.
- When changing the JSON schema, update `README.md`, `data/world.json`, loader validation, and loader tests together.

## Verification

Run these commands after code changes:

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --no-deps
```

Add or update tests for behavior changes. Unit tests should stay near the module they cover; end-to-end actor behavior belongs in `src/server/tests.rs`.

## Change discipline

- Keep patches scoped to the requested behavior.
- Do not rewrite unrelated code or discard existing local changes.
- Treat `Cargo.lock` as part of the repository and update it when dependencies change.
- Update the README when commands, configuration, setup steps, architecture, or the world format changes.
