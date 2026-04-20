//! DAG validation: Kahn topological sort + cycle-edge reconstruction.
//!
//! Inputs are already-deduplicated `(id, deps)` pairs so this module only
//! handles graph topology. Missing-dep detection stays in the caller.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Result of a topological sort attempt.
#[derive(Debug)]
pub enum TopoOutcome {
    /// Tasks in a valid topological order.
    Ordered(Vec<String>),
    /// A cycle was detected. `remaining` is the set of nodes that could not
    /// be scheduled; `edges` is a list of `(from, to)` dependency edges
    /// participating in the cycle.
    Cycle {
        remaining: Vec<String>,
        edges: Vec<(String, String)>,
    },
}

/// Run Kahn's algorithm. `deps[id]` is the set of ids `id` depends on
/// (i.e. incoming edges in the "dep -> task" orientation).
#[must_use]
pub fn topo_sort(deps: &BTreeMap<String, Vec<String>>) -> TopoOutcome {
    let ids: BTreeSet<_> = deps.keys().cloned().collect();
    let mut indegree: BTreeMap<String, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
    // "dep -> task" adjacency so we can decrement successors when a dep resolves.
    let mut succ: BTreeMap<String, Vec<String>> =
        ids.iter().map(|id| (id.clone(), Vec::new())).collect();

    for (id, edges) in deps {
        for dep in edges {
            if ids.contains(dep) {
                *indegree.entry(id.clone()).or_insert(0) += 1;
                succ.entry(dep.clone()).or_default().push(id.clone());
            }
        }
    }

    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter(|(_, &n)| n == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut ordered = Vec::with_capacity(ids.len());
    while let Some(id) = queue.pop_front() {
        ordered.push(id.clone());
        if let Some(children) = succ.get(&id) {
            for child in children {
                let entry = indegree.get_mut(child).expect("indegree entry");
                *entry -= 1;
                if *entry == 0 {
                    queue.push_back(child.clone());
                }
            }
        }
    }

    if ordered.len() == ids.len() {
        return TopoOutcome::Ordered(ordered);
    }

    // Cycle branch. Collect the ids that never scheduled and the edges
    // wholly within that set.
    let scheduled: BTreeSet<_> = ordered.into_iter().collect();
    let remaining: Vec<String> = ids
        .iter()
        .filter(|id| !scheduled.contains(*id))
        .cloned()
        .collect();
    let remaining_set: BTreeSet<_> = remaining.iter().cloned().collect();

    let mut edges: Vec<(String, String)> = Vec::new();
    for id in &remaining {
        if let Some(ds) = deps.get(id) {
            for dep in ds {
                if remaining_set.contains(dep) {
                    edges.push((dep.clone(), id.clone()));
                }
            }
        }
    }

    TopoOutcome::Cycle { remaining, edges }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deps(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(id, ds)| {
                (
                    (*id).to_string(),
                    ds.iter().map(|d| (*d).to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn linear_dag_orders() {
        let g = deps(&[("T1", &[]), ("T2", &["T1"]), ("T3", &["T2"])]);
        match topo_sort(&g) {
            TopoOutcome::Ordered(order) => assert_eq!(order, vec!["T1", "T2", "T3"]),
            TopoOutcome::Cycle { .. } => panic!("expected ordered"),
        }
    }

    #[test]
    fn diamond_dag_orders() {
        let g = deps(&[
            ("T1", &[]),
            ("T2", &["T1"]),
            ("T3", &["T1"]),
            ("T4", &["T2", "T3"]),
        ]);
        match topo_sort(&g) {
            TopoOutcome::Ordered(order) => {
                assert_eq!(order[0], "T1");
                assert_eq!(order[3], "T4");
            }
            TopoOutcome::Cycle { .. } => panic!("expected ordered"),
        }
    }

    #[test]
    fn simple_cycle_detected() {
        let g = deps(&[("T1", &["T2"]), ("T2", &["T1"])]);
        match topo_sort(&g) {
            TopoOutcome::Cycle { remaining, edges } => {
                assert!(remaining.contains(&"T1".to_string()));
                assert!(remaining.contains(&"T2".to_string()));
                assert_eq!(edges.len(), 2);
            }
            TopoOutcome::Ordered(_) => panic!("expected cycle"),
        }
    }
}
