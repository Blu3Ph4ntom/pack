//! Install orchestration.

use pack_core::{InstallPlan, PackResult};

pub struct Installer;

impl Installer {
    pub fn new() -> Self {
        Self
    }

    pub fn install(&self, plan: &InstallPlan) -> PackResult<()> {
        let _ = plan;
        Ok(())
    }
}

impl Default for Installer {
    fn default() -> Self {
        Self::new()
    }
}
