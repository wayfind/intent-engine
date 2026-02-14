//! Pure validation functions for plan execution.
//!
//! Shared between SQLite `PlanExecutor` and Neo4j `Neo4jPlanExecutor`.
//! All functions are stateless — no database access.

use crate::error::{IntentError, Result};
use crate::plan::{FlatTask, TaskStatus};
use std::collections::{HashMap, HashSet};

/// Validate that all dependency references exist within the plan.
pub fn validate_dependencies(flat_tasks: &[FlatTask]) -> Result<()> {
    let task_names: HashSet<&str> = flat_tasks
        .iter()
        .filter_map(|t| t.name.as_deref())
        .collect();

    for task in flat_tasks {
        for dep_name in &task.depends_on {
            if !task_names.contains(dep_name.as_str()) {
                let task_name = task.name.as_deref().unwrap_or("<unknown>");
                return Err(IntentError::InvalidInput(format!(
                    "Task '{}' depends on '{}', but '{}' is not in the plan",
                    task_name, dep_name, dep_name
                )));
            }
        }
    }

    Ok(())
}

/// Validate that at most one task in the batch has status='doing'.
pub fn validate_batch_single_doing(flat_tasks: &[FlatTask]) -> Result<()> {
    let doing_tasks: Vec<&FlatTask> = flat_tasks
        .iter()
        .filter(|task| matches!(task.status, Some(TaskStatus::Doing)))
        .collect();

    if doing_tasks.len() > 1 {
        let names: Vec<&str> = doing_tasks
            .iter()
            .map(|t| t.name.as_deref().unwrap_or("<unknown>"))
            .collect();
        return Err(IntentError::InvalidInput(format!(
            "Batch single doing constraint violated: only one task per batch can have status='doing'. Found: {}",
            names.join(", ")
        )));
    }

    Ok(())
}

/// Detect circular dependencies using Tarjan's SCC algorithm.
pub fn detect_circular_dependencies(flat_tasks: &[FlatTask]) -> Result<()> {
    if flat_tasks.is_empty() {
        return Ok(());
    }

    let name_to_idx: HashMap<&str, usize> = flat_tasks
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.name.as_ref().map(|n| (n.as_str(), i)))
        .collect();

    // Build adjacency list
    let mut graph: Vec<Vec<usize>> = vec![Vec::new(); flat_tasks.len()];
    for (idx, task) in flat_tasks.iter().enumerate() {
        for dep_name in &task.depends_on {
            if let Some(&dep_idx) = name_to_idx.get(dep_name.as_str()) {
                graph[idx].push(dep_idx);
            }
        }
    }

    // Check self-loops first
    for task in flat_tasks {
        if let Some(name) = &task.name {
            if task.depends_on.contains(name) {
                return Err(IntentError::InvalidInput(format!(
                    "Circular dependency detected: task '{}' depends on itself",
                    name
                )));
            }
        }
    }

    // Tarjan's SCC
    let sccs = tarjan_scc(&graph);
    for scc in sccs {
        if scc.len() > 1 {
            let cycle_names: Vec<&str> = scc
                .iter()
                .map(|&idx| flat_tasks[idx].name.as_deref().unwrap_or("<unknown>"))
                .collect();
            return Err(IntentError::InvalidInput(format!(
                "Circular dependency detected: {}",
                cycle_names.join(" → ")
            )));
        }
    }

    Ok(())
}

/// Tarjan's algorithm for finding strongly connected components.
///
/// Returns a list of SCCs, where each SCC is a list of node indices.
/// A cycle exists if any SCC has more than one node.
pub fn tarjan_scc(graph: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = graph.len();
    let mut index_counter = 0;
    let mut stack = Vec::new();
    let mut on_stack = vec![false; n];
    let mut index = vec![usize::MAX; n];
    let mut lowlink = vec![0; n];
    let mut result = Vec::new();

    fn strongconnect(
        v: usize,
        graph: &[Vec<usize>],
        index_counter: &mut usize,
        stack: &mut Vec<usize>,
        on_stack: &mut Vec<bool>,
        index: &mut Vec<usize>,
        lowlink: &mut Vec<usize>,
        result: &mut Vec<Vec<usize>>,
    ) {
        index[v] = *index_counter;
        lowlink[v] = *index_counter;
        *index_counter += 1;
        stack.push(v);
        on_stack[v] = true;

        for &w in &graph[v] {
            if index[w] == usize::MAX {
                strongconnect(
                    w,
                    graph,
                    index_counter,
                    stack,
                    on_stack,
                    index,
                    lowlink,
                    result,
                );
                lowlink[v] = lowlink[v].min(lowlink[w]);
            } else if on_stack[w] {
                lowlink[v] = lowlink[v].min(index[w]);
            }
        }

        if lowlink[v] == index[v] {
            let mut component = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack[w] = false;
                component.push(w);
                if w == v {
                    break;
                }
            }
            result.push(component);
        }
    }

    for v in 0..n {
        if index[v] == usize::MAX {
            strongconnect(
                v,
                graph,
                &mut index_counter,
                &mut stack,
                &mut on_stack,
                &mut index,
                &mut lowlink,
                &mut result,
            );
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_batch_single_doing_ok() {
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                status: Some(TaskStatus::Doing),
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                status: Some(TaskStatus::Todo),
                ..Default::default()
            },
        ];
        assert!(validate_batch_single_doing(&tasks).is_ok());
    }

    #[test]
    fn test_validate_batch_single_doing_violation() {
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                status: Some(TaskStatus::Doing),
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                status: Some(TaskStatus::Doing),
                ..Default::default()
            },
        ];
        assert!(validate_batch_single_doing(&tasks).is_err());
    }

    #[test]
    fn test_validate_batch_single_doing_zero() {
        let tasks = vec![FlatTask {
            name: Some("A".to_string()),
            status: Some(TaskStatus::Todo),
            ..Default::default()
        }];
        assert!(validate_batch_single_doing(&tasks).is_ok());
    }

    #[test]
    fn test_validate_dependencies_ok() {
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                depends_on: vec!["B".to_string()],
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                ..Default::default()
            },
        ];
        assert!(validate_dependencies(&tasks).is_ok());
    }

    #[test]
    fn test_validate_dependencies_missing() {
        let tasks = vec![FlatTask {
            name: Some("A".to_string()),
            depends_on: vec!["NonExistent".to_string()],
            ..Default::default()
        }];
        let err = validate_dependencies(&tasks).unwrap_err();
        assert!(err.to_string().contains("NonExistent"));
    }

    #[test]
    fn test_validate_dependencies_empty() {
        assert!(validate_dependencies(&[]).is_ok());
    }

    #[test]
    fn test_detect_circular_no_cycle() {
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                depends_on: vec!["B".to_string()],
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                ..Default::default()
            },
        ];
        assert!(detect_circular_dependencies(&tasks).is_ok());
    }

    #[test]
    fn test_detect_circular_self_loop() {
        let tasks = vec![FlatTask {
            name: Some("A".to_string()),
            depends_on: vec!["A".to_string()],
            ..Default::default()
        }];
        let err = detect_circular_dependencies(&tasks).unwrap_err();
        assert!(err.to_string().contains("depends on itself"));
    }

    #[test]
    fn test_detect_circular_two_node_cycle() {
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                depends_on: vec!["B".to_string()],
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                depends_on: vec!["A".to_string()],
                ..Default::default()
            },
        ];
        assert!(detect_circular_dependencies(&tasks).is_err());
    }

    #[test]
    fn test_detect_circular_three_node_cycle() {
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                depends_on: vec!["B".to_string()],
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                depends_on: vec!["C".to_string()],
                ..Default::default()
            },
            FlatTask {
                name: Some("C".to_string()),
                depends_on: vec!["A".to_string()],
                ..Default::default()
            },
        ];
        assert!(detect_circular_dependencies(&tasks).is_err());
    }

    #[test]
    fn test_detect_circular_empty() {
        assert!(detect_circular_dependencies(&[]).is_ok());
    }

    #[test]
    fn test_tarjan_scc_no_cycle() {
        let graph = vec![vec![1], vec![]]; // A → B
        let sccs = tarjan_scc(&graph);
        assert!(sccs.iter().all(|scc| scc.len() == 1));
    }

    #[test]
    fn test_tarjan_scc_with_cycle() {
        let graph = vec![vec![1], vec![0]]; // A → B → A
        let sccs = tarjan_scc(&graph);
        assert!(sccs.iter().any(|scc| scc.len() > 1));
    }

    #[test]
    fn test_tarjan_scc_empty() {
        let graph: Vec<Vec<usize>> = vec![];
        let sccs = tarjan_scc(&graph);
        assert!(sccs.is_empty());
    }

    #[test]
    fn test_tarjan_scc_disconnected() {
        let graph = vec![vec![], vec![]]; // A, B (no edges)
        let sccs = tarjan_scc(&graph);
        assert_eq!(sccs.len(), 2);
        assert!(sccs.iter().all(|scc| scc.len() == 1));
    }
}
