mod look;
mod move_;

use crate::game::{commands::look::Look, registry::CommandRegistry};

pub fn register_all(registry: &mut CommandRegistry) -> Result<(), String> {
    registry.register(Look);

    Ok(())
}