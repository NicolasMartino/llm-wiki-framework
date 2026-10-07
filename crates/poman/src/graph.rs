//! The `Blocked by` graph: each deadline file and the files it waits on, and
//! the loops they form.

use std::collections::{BTreeMap, BTreeSet};

/// Each file, from the repository root, and the files it waits on.
pub type Graph = BTreeMap<String, Vec<String>>;

/// Every file `from` reaches in one step or more.
fn reach<'graph>(graph: &'graph Graph, from: &str) -> BTreeSet<&'graph str> {
    let mut reached = BTreeSet::new();
    let mut stack: Vec<&str> = graph
        .get(from)
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
    while let Some(file) = stack.pop() {
        if reached.insert(file) {
            stack.extend(graph.get(file).into_iter().flatten().map(String::as_str));
        }
    }
    reached
}

/// The loops: each group of files that all reach one another, sorted, the
/// groups in the order of their first file.
#[must_use]
pub fn loops(graph: &Graph) -> Vec<Vec<String>> {
    let mut placed: BTreeSet<String> = BTreeSet::new();
    let mut found = Vec::new();
    for file in graph.keys() {
        if placed.contains(file) {
            continue;
        }
        let reached = reach(graph, file);
        if !reached.contains(file.as_str()) {
            continue;
        }
        let group: Vec<String> = reached
            .into_iter()
            .filter(|other| reach(graph, other).contains(file.as_str()))
            .map(str::to_owned)
            .collect();
        placed.extend(group.iter().cloned());
        found.push(group);
    }
    found
}

/// The files a walk from `from` back to it goes through, `from` first and
/// last, when `from` is in a loop.
#[must_use]
pub fn loop_through(graph: &Graph, from: &str) -> Option<Vec<String>> {
    let mut came_from: BTreeMap<&str, &str> = BTreeMap::new();
    let mut queue: Vec<&str> = vec![from];
    let mut next = 0;
    while let Some(&file) = queue.get(next) {
        next += 1;
        for blocker in graph.get(file).into_iter().flatten() {
            if blocker == from {
                let mut walk = vec![from.to_owned()];
                let mut step = file;
                while step != from {
                    walk.push(step.to_owned());
                    step = came_from.get(step).copied().unwrap_or(from);
                }
                walk.push(from.to_owned());
                walk.reverse();
                return Some(walk);
            }
            if !came_from.contains_key(blocker.as_str()) {
                came_from.insert(blocker, file);
                queue.push(blocker);
            }
        }
    }
    None
}
