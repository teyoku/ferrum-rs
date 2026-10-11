use std::fmt::Debug;

use crate::{ServerActor, server::SessionId};

pub type CommandResult = Result<(), &'static str>;

pub trait Command: Debug + Sync + Send {
    fn name(&self) -> &'static str;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> CommandResult;
}

pub struct CommandContext<'a> {
    pub session_id: SessionId,
    pub args: &'a str,
    server: &'a mut ServerActor,
}

impl<'a> CommandContext<'a> {
    pub fn new(session_id: SessionId, args: &'a str, server: &'a mut ServerActor) -> Self {
        Self {
            session_id,
            args,
            server,
        }
    }

    pub fn reply(&mut self, text: impl Into<String>) {
        self.server.send_text(self.session_id, text.into());
    }

    pub fn online_count(&self) -> usize {
        self.server.online_count()
    }

    pub fn say(&mut self, message: &str) {
        self.server.broadcast_from(self.session_id, message);
    }

    pub fn close(&mut self) {
        self.server.close_session(self.session_id);
    }
}
