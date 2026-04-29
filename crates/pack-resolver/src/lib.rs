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

#[cfg(test)]
mod tests {
    use super::*;
    use pack_core::GemVersion;

    #[test]
    fn test_resolver_new() {
        let _resolver = Resolver::new();
        assert!(true);
    }

    #[test]
    fn test_resolver_default() {
        let _resolver = Resolver::default();
        assert!(true);
    }

    #[test]
    fn test_resolve_empty() {
        let resolver = Resolver::new();
        let deps: Vec<Dependency> = vec![];
        let result = resolver.resolve(&deps, &[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_resolve_single_dep() {
        let resolver = Resolver::new();
        let deps = vec![Dependency {
            name: GemName("rails".to_string()),
            version: Some(GemVersion("7.1.0".to_string())),
            group: None,
        }];
        let result = resolver.resolve(&deps, &[]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "rails");
    }

    #[test]
    fn test_resolve_multiple_deps() {
        let resolver = Resolver::new();
        let deps = vec![
            Dependency {
                name: GemName("rails".to_string()),
                version: Some(GemVersion("7.1.0".to_string())),
                group: None,
            },
            Dependency {
                name: GemName("puma".to_string()),
                version: None,
                group: None,
            },
        ];
        let result = resolver.resolve(&deps, &[]).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "rails");
        assert_eq!(result[1].0, "puma");
    }
}
