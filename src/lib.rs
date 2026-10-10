//! Ferrum MUD server library.
//!
//! The library separates connection handling from shared server state:
//! `session` owns individual TCP connections, while `server` owns the actor
//! that coordinates all connected sessions.

pub mod server;
pub mod session;

// Re-export the main API so the binary does not depend on the module layout.
pub use server::{ServerActor, ServerHandle};

pub use session::process_client;
