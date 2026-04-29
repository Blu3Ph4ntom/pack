//! Dependency resolution.

use pack_core::{Dependency, GemName, PackResult};

pub struct Resolver;

impl Resolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve(
        &self,
        deps: &[Dependency],
        _lock_deps: &[Dependency],
    ) -> PackResult<Vec<GemName>> {
        Ok(deps.iter().map(|d| d.name.clone()).collect())
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}
