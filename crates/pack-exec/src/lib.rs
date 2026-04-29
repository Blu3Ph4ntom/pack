//! Command execution.

use pack_core::PackResult;

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Self
    }

    pub fn exec(&self, cmd: &str, args: &[String]) -> PackResult<()> {
        let _ = (cmd, args);
        Ok(())
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
