use std::{collections::HashMap, error::Error, fmt, net::SocketAddr};

use tokio::sync::{mpsc, oneshot};

/// Maximum number of commands waiting in the server actor's mailbox.
pub const SERVER_MAILBOX_CAPACITY: usize = 256;

/// Maximum number of outbound events waiting for one client session.
pub const SESSION_MAILBOX_CAPACITY: usize = 32;

/// Identifier assigned to a connection for the duration of its session.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SessionId(u64);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Event sent by the server actor to an individual client task.
#[derive(Debug)]
pub enum SessionEvent {
    /// Write text to the client's TCP connection.
    Text(String),

    /// Finish the client session gracefully.
    Close,
}

/// Internal messages accepted by the server actor.
///
/// This type stays private so callers must use `ServerHandle`, which keeps
/// mailbox access and error handling behind a small public API.
enum ServerCommand {
    Connect {
        address: SocketAddr,
        output: mpsc::Sender<SessionEvent>,
        // A one-shot channel returns the newly allocated session ID to the
        // connection task that requested registration.
        reply: oneshot::Sender<SessionId>,
    },
    Input {
        session_id: SessionId,
        line: String,
    },
    Disconnect {
        session_id: SessionId,
    },
}

/// Cloneable address through which tasks communicate with `ServerActor`.
#[derive(Debug, Clone)]
pub struct ServerHandle {
    sender: mpsc::Sender<ServerCommand>,
}

/// Actor that exclusively owns and coordinates all connected sessions.
///
/// Because this state is accessed only inside the actor's mailbox loop, it
/// does not need to be wrapped in `Arc<Mutex<_>>`.
#[derive(Debug)]
pub struct ServerActor {
    sessions: HashMap<SessionId, ConnectedSession>,
    next_session_id: u64,
    receiver: mpsc::Receiver<ServerCommand>,
}

/// Server-side routing information for one connected client.
#[derive(Debug)]
pub struct ConnectedSession {
    address: SocketAddr,
    output: mpsc::Sender<SessionEvent>,
}

/// Returned when a handle can no longer reach the server actor.
#[derive(Debug, Clone, Copy)]
pub struct ServerStopped;

impl fmt::Display for ServerStopped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("server actor has stopped")
    }
}

impl Error for ServerStopped {}

impl ServerActor {
    /// Starts the actor as a Tokio task and returns its communication handle.
    pub fn spawn() -> ServerHandle {
        let (sender, receiver) = mpsc::channel(SERVER_MAILBOX_CAPACITY);

        let actor = Self {
            sessions: HashMap::new(),
            next_session_id: 1,
            receiver,
        };

        tokio::spawn(actor.run());

        ServerHandle { sender }
    }

    async fn run(mut self) {
        // Receiving commands sequentially gives every mutation of `sessions`
        // a deterministic order and preserves the actor's exclusive ownership.
        while let Some(command) = self.receiver.recv().await {
            match command {
                ServerCommand::Connect {
                    address,
                    output,
                    reply,
                } => self.connect(address, output, reply),
                ServerCommand::Input { session_id, line } => {
                    self.handle_input(session_id, &line);
                }
                ServerCommand::Disconnect { session_id } => {
                    self.sessions.remove(&session_id);
                }
            }
        }
    }

    fn connect(
        &mut self,
        address: SocketAddr,
        output: mpsc::Sender<SessionEvent>,
        reply: oneshot::Sender<SessionId>,
    ) {
        let session_id = SessionId(self.next_session_id);

        // Checked arithmetic makes an impossible-in-practice ID wraparound
        // fail explicitly instead of silently reusing an active identifier.
        self.next_session_id = self
            .next_session_id
            .checked_add(1)
            .expect("session identifier overflowed");

        self.sessions
            .insert(session_id, ConnectedSession { address, output });

        // The connection task may disappear before receiving the response.
        // In that case, remove the registration immediately.
        if reply.send(session_id).is_err() {
            self.sessions.remove(&session_id);
        }
    }

    fn handle_input(&mut self, session_id: SessionId, line: &str) {
        let line = line.trim();

        if line.eq_ignore_ascii_case("quit") {
            self.send_to(session_id, SessionEvent::Close);
            return;
        }

        if line.eq_ignore_ascii_case("who") {
            let count = self.sessions.len();

            self.send_to(
                session_id,
                SessionEvent::Text(format!("Players online: {count}\r\n")),
            );

            return;
        }

        if let Some(message) = line.strip_prefix("say ") {
            let message = message.trim();

            if message.is_empty() {
                self.send_to(
                    session_id,
                    SessionEvent::Text("What do you want to say?\r\n".to_owned()),
                );
            } else {
                self.broadcast(session_id, message);
            }

            return;
        }

        self.send_to(
            session_id,
            SessionEvent::Text(format!("You said: {line}\r\n")),
        );
    }

    fn send_to(&mut self, session_id: SessionId, event: SessionEvent) {
        // `try_send` prevents a slow client from blocking the entire actor.
        // A full or closed client mailbox is treated as a dead session.
        let should_remove = self
            .sessions
            .get(&session_id)
            .is_some_and(|session| session.output.try_send(event).is_err());

        if should_remove {
            self.sessions.remove(&session_id);
        }
    }

    fn broadcast(&mut self, sender_id: SessionId, message: &str) {
        let Some(sender_address) = self.sessions.get(&sender_id).map(|session| session.address)
        else {
            return;
        };

        let mut disconnected = Vec::new();

        // Sessions cannot be removed while the map is being iterated, so
        // failed recipients are collected and removed in a second pass.
        for (&session_id, session) in &self.sessions {
            let text = if session_id == sender_id {
                format!("You say: {message}\r\n")
            } else {
                format!("{sender_address} says: {message}\r\n")
            };

            if session.output.try_send(SessionEvent::Text(text)).is_err() {
                disconnected.push(session_id);
            }
        }

        for session_id in disconnected {
            self.sessions.remove(&session_id);
        }
    }
}

impl ServerHandle {
    /// Registers a new connection and waits for its actor-assigned ID.
    pub async fn connect(
        &self,
        address: SocketAddr,
        output: mpsc::Sender<SessionEvent>,
    ) -> Result<SessionId, ServerStopped> {
        // Unlike mpsc, a one-shot channel represents exactly one response to
        // exactly one request.
        let (reply, response) = oneshot::channel();

        self.sender
            .send(ServerCommand::Connect {
                address,
                output,
                reply,
            })
            .await
            .map_err(|_| ServerStopped)?;

        response.await.map_err(|_| ServerStopped)
    }

    /// Forwards one line of client input to the server actor.
    pub async fn input(&self, session_id: SessionId, line: String) -> Result<(), ServerStopped> {
        self.sender
            .send(ServerCommand::Input { session_id, line })
            .await
            .map_err(|_| ServerStopped)
    }

    /// Removes a session from the actor's routing table.
    pub async fn disconnect(&self, session_id: SessionId) -> Result<(), ServerStopped> {
        self.sender
            .send(ServerCommand::Disconnect { session_id })
            .await
            .map_err(|_| ServerStopped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reports_connected_session_count() {
        let server = ServerActor::spawn();

        let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let address = "127.0.0.1:4000".parse().unwrap();

        let session_id = server.connect(address, output).await.unwrap();

        server.input(session_id, "who".to_owned()).await.unwrap();

        assert!(matches!(
            events.recv().await,
            Some(SessionEvent::Text(text))
                if text == "Players online: 1\r\n"
        ));
    }

    #[tokio::test]
    async fn quit_closes_the_session() {
        let server = ServerActor::spawn();

        let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

        let address = "127.0.0.1:4000".parse().unwrap();

        let session_id = server.connect(address, output).await.unwrap();

        server.input(session_id, "quit".to_owned()).await.unwrap();

        assert!(matches!(events.recv().await, Some(SessionEvent::Close)));
    }
}
