use must_core::{Error, Result};
use std::collections::{HashMap, HashSet, VecDeque};

/// A wave is a set of recipe names that can run concurrently.
pub type Wave = Vec<String>;

/// A directed acyclic graph of recipe dependencies.
pub struct Dag {
    /// All nodes (recipe names)
    nodes: Vec<String>,
    /// Adjacency list: node -> list of nodes it depends on
    deps: HashMap<String, Vec<String>>,
}

impl Dag {
    /// Build a DAG from a map of recipe_name -> deps list.
    pub fn new(recipes: HashMap<String, Vec<String>>) -> Self {
        let nodes = recipes.keys().cloned().collect();
        Dag {
            nodes,
            deps: recipes,
        }
    }

    /// Return recipes in topological order (dependencies before dependents).
    /// Raises Error::CycleDetected if the graph has cycles.
    pub fn topo_sort(&self) -> Result<Vec<String>> {
        let waves = self.waves()?;
        Ok(waves.into_iter().flatten().collect())
    }

    /// Group recipes into waves for parallel execution.
    /// All recipes in a wave can run concurrently; a wave only starts
    /// after all previous waves complete.
    pub fn waves(&self) -> Result<Vec<Wave>> {
        // in_degree[node] = number of unresolved dependencies
        let mut in_deg: HashMap<&str, usize> = self
            .nodes
            .iter()
            .map(|n| (n.as_str(), self.deps.get(n).map(|d| d.len()).unwrap_or(0)))
            .collect();

        // blocked_by[dep] = nodes that depend on dep (reverse edges)
        let mut blocked_by: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            blocked_by.entry(node).or_default();
        }
        for (node, deps) in &self.deps {
            for dep in deps {
                blocked_by
                    .entry(dep.as_str())
                    .or_default()
                    .push(node.as_str());
            }
        }

        // Start with nodes that have no dependencies
        let mut queue: VecDeque<&str> = in_deg
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&n, _)| n)
            .collect();

        let mut waves: Vec<Wave> = Vec::new();
        let mut visited = 0usize;

        while !queue.is_empty() {
            let mut wave: Wave = queue.drain(..).map(|s| s.to_string()).collect();
            wave.sort();
            visited += wave.len();

            let mut next_queue: VecDeque<&str> = VecDeque::new();
            for node in &wave {
                if let Some(blocked) = blocked_by.get(node.as_str()) {
                    for &dependent in blocked {
                        let deg = in_deg.get_mut(dependent).unwrap();
                        *deg -= 1;
                        if *deg == 0 {
                            next_queue.push_back(dependent);
                        }
                    }
                }
            }
            waves.push(wave);
            queue = next_queue;
        }

        if visited != self.nodes.len() {
            let remaining: Vec<&str> = in_deg
                .iter()
                .filter(|(_, &d)| d > 0)
                .map(|(&n, _)| n)
                .collect();
            let cycle = find_cycle(&self.deps, &remaining);
            return Err(Error::CycleDetected { cycle });
        }

        Ok(waves)
    }

    /// Return all recipes reachable from the given starting recipe (including itself).
    pub fn reachable_from(&self, start: &str) -> Result<Vec<String>> {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if visited.insert(node) {
                if let Some(deps) = self.deps.get(node) {
                    for dep in deps {
                        stack.push(dep);
                    }
                }
            }
        }
        // Return in topo order
        let all = self.topo_sort()?;
        Ok(all
            .into_iter()
            .filter(|n| visited.contains(n.as_str()))
            .collect())
    }
}

fn find_cycle(deps: &HashMap<String, Vec<String>>, candidates: &[&str]) -> String {
    // DFS to find a cycle among candidates
    let candidate_set: HashSet<&str> = candidates.iter().cloned().collect();
    let mut path: Vec<&str> = Vec::new();
    let mut on_path: HashSet<&str> = HashSet::new();

    for &start in candidates {
        path.clear();
        on_path.clear();
        if let Some(cycle) = dfs_cycle(start, deps, &candidate_set, &mut path, &mut on_path) {
            return cycle;
        }
    }
    candidates.join(" -> ") // fallback
}

fn dfs_cycle<'a>(
    node: &'a str,
    deps: &'a HashMap<String, Vec<String>>,
    candidates: &HashSet<&str>,
    path: &mut Vec<&'a str>,
    on_path: &mut HashSet<&'a str>,
) -> Option<String> {
    if on_path.contains(node) {
        // Found cycle — reconstruct it
        let cycle_start = path.iter().position(|&n| n == node).unwrap_or(0);
        let mut cycle: Vec<&str> = path[cycle_start..].to_vec();
        cycle.push(node);
        return Some(cycle.join(" -> "));
    }
    if !candidates.contains(node) {
        return None;
    }
    on_path.insert(node);
    path.push(node);
    if let Some(node_deps) = deps.get(node) {
        for dep in node_deps {
            if let Some(cycle) = dfs_cycle(dep, deps, candidates, path, on_path) {
                return Some(cycle);
            }
        }
    }
    path.pop();
    on_path.remove(node);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deps(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    #[test]
    fn test_topo_sort_simple() {
        let dag = Dag::new(deps(&[("a", &[]), ("b", &["a"]), ("c", &["b"])]));
        let order = dag.topo_sort().unwrap();
        let a = order.iter().position(|x| x == "a").unwrap();
        let b = order.iter().position(|x| x == "b").unwrap();
        let c = order.iter().position(|x| x == "c").unwrap();
        assert!(a < b && b < c);
    }

    #[test]
    fn test_waves() {
        let dag = Dag::new(deps(&[
            ("codegen", &[]),
            ("serve", &[]),
            ("build", &["codegen"]),
            ("test", &["codegen"]),
            ("release", &["build", "test", "serve"]),
        ]));
        let waves = dag.waves().unwrap();
        assert_eq!(waves.len(), 3);
        // Wave 1: codegen, serve (independent)
        assert!(waves[0].contains(&"codegen".to_string()));
        assert!(waves[0].contains(&"serve".to_string()));
        // Wave 2: build, test
        assert!(waves[1].contains(&"build".to_string()));
        assert!(waves[1].contains(&"test".to_string()));
        // Wave 3: release
        assert_eq!(waves[2], vec!["release".to_string()]);
    }

    #[test]
    fn test_cycle_detection() {
        let dag = Dag::new(deps(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])]));
        let result = dag.topo_sort();
        assert!(matches!(result, Err(Error::CycleDetected { .. })));
    }
}
