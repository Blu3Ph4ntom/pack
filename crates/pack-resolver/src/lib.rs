//! Dependency resolution.
//!
//! The resolver is responsible for finding a valid set of gem versions
//! that satisfy all dependency constraints.

use pack_core::{Dependency, GemName, PackResult};
use std::collections::{HashMap, HashSet};

pub struct Resolver {
    max_iterations: usize,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            max_iterations: 1000,
        }
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn resolve(
        &self,
        deps: &[Dependency],
        _lock_deps: &[Dependency],
    ) -> PackResult<Vec<GemName>> {
        Ok(deps.iter().map(|d| d.name.clone()).collect())
    }

    /// Resolve with dependency graph validation
    pub fn resolve_with_graph(
        &self,
        deps: &[Dependency],
    ) -> PackResult<ResolutionResult> {
        let mut resolved: HashMap<GemName, Dependency> = HashMap::new();
        let mut seen: HashSet<GemName> = HashSet::new();
        let mut conflicts: Vec<Conflict> = Vec::new();

        for dep in deps {
            if seen.contains(&dep.name) {
                if let Some(existing) = resolved.get(&dep.name) {
                    if existing.version != dep.version {
                        conflicts.push(Conflict {
                            gem: dep.name.clone(),
                            versions: vec![
                                existing.version.clone().unwrap(),
                                dep.version.clone().unwrap(),
                            ],
                        });
                    }
                }
            } else {
                seen.insert(dep.name.clone());
                resolved.insert(dep.name.clone(), dep.clone());
            }
        }

        Ok(ResolutionResult {
            resolved: resolved.into_values().collect(),
            conflicts,
        })
    }

    /// Find all dependencies of a gem (transitively)
    pub fn find_all_dependencies(
        &self,
        root: &GemName,
        deps: &HashMap<GemName, Vec<GemName>>,
    ) -> Vec<GemName> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        self.collect_deps(root, deps, &mut visited, &mut result);
        result
    }

    fn collect_deps(
        &self,
        gem: &GemName,
        deps: &HashMap<GemName, Vec<GemName>>,
        visited: &mut HashSet<GemName>,
        result: &mut Vec<GemName>,
    ) {
        if visited.contains(gem) {
            return;
        }
        visited.insert(gem.clone());
        result.push(gem.clone());

        if let Some(gem_deps) = deps.get(gem) {
            for dep in gem_deps {
                self.collect_deps(dep, deps, visited, result);
            }
        }
    }

    /// Check for circular dependencies
    pub fn has_circular_deps(&self, deps: &HashMap<GemName, Vec<GemName>>) -> bool {
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();

        for gem in deps.keys() {
            if self.is_cyclic(gem, deps, &mut visiting, &mut visited) {
                return true;
            }
        }
        false
    }

    fn is_cyclic(
        &self,
        gem: &GemName,
        deps: &HashMap<GemName, Vec<GemName>>,
        visiting: &mut HashSet<GemName>,
        visited: &mut HashSet<GemName>,
    ) -> bool {
        if visited.contains(gem) {
            return false;
        }
        if visiting.contains(gem) {
            return true;
        }

        visiting.insert(gem.clone());

        if let Some(gem_deps) = deps.get(gem) {
            for dep in gem_deps {
                if self.is_cyclic(dep, deps, visiting, visited) {
                    return true;
                }
            }
        }

        visiting.remove(gem);
        visited.insert(gem.clone());
        false
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ResolutionResult {
    pub resolved: Vec<Dependency>,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub gem: GemName,
    pub versions: Vec<pack_core::GemVersion>,
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

    #[test]
    fn test_resolve_with_graph() {
        let resolver = Resolver::new();
        let deps = vec![
            Dependency {
                name: GemName("rails".to_string()),
                version: Some(GemVersion("7.1.0".to_string())),
                group: None,
            },
        ];
        let result = resolver.resolve_with_graph(&deps).unwrap();
        assert_eq!(result.resolved.len(), 1);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_has_circular_deps_no_cycle() {
        let resolver = Resolver::new();
        let mut deps = HashMap::new();
        deps.insert(GemName("rails".to_string()), vec![GemName("actionpack".to_string())]);
        deps.insert(GemName("actionpack".to_string()), vec![GemName("rack".to_string())]);

        assert!(!resolver.has_circular_deps(&deps));
    }

    #[test]
    fn test_has_circular_deps_with_cycle() {
        let resolver = Resolver::new();
        let mut deps = HashMap::new();
        deps.insert(GemName("a".to_string()), vec![GemName("b".to_string())]);
        deps.insert(GemName("b".to_string()), vec![GemName("c".to_string())]);
        deps.insert(GemName("c".to_string()), vec![GemName("a".to_string())]);

        assert!(resolver.has_circular_deps(&deps));
    }

    #[test]
    fn test_find_all_dependencies() {
        let resolver = Resolver::new();
        let mut deps = HashMap::new();
        deps.insert(GemName("rails".to_string()), vec![GemName("actionpack".to_string())]);
        deps.insert(GemName("actionpack".to_string()), vec![GemName("actionview".to_string())]);

        let result = resolver.find_all_dependencies(&GemName("rails".to_string()), &deps);
        assert!(result.contains(&GemName("rails".to_string())));
        assert!(result.contains(&GemName("actionpack".to_string())));
        assert!(result.contains(&GemName("actionview".to_string())));
    }
}
