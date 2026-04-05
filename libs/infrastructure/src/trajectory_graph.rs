/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
/*
 * Aiome - Trajectory Graph Construction
 */

use aiome_core::error::AiomeError;
use aiome_core::trajectory::TrajectoryStep;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: u32,
    pub step: TrajectoryStep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: u32,
    pub to: u32,
}

const MAX_NODES: usize = 1000;
const MAX_DEPTH: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl TrajectoryGraph {
    pub fn build_graph(steps: Vec<TrajectoryStep>) -> Result<Self, AiomeError> {
        if steps.len() > MAX_NODES {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Trajectory too large: {} steps exceeds limit of {}",
                    steps.len(),
                    MAX_NODES
                ),
            });
        }
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut id_map = HashMap::new();

        // 1. Create nodes and map IDs
        for step in steps {
            let id = step.step_id;
            id_map.insert(id, step.clone());
            nodes.push(GraphNode { id, step });
        }

        // 2. Create edges and check for circularity/invalid parents
        for node in &nodes {
            if let Some(parent_id) = node.step.parent_step_id {
                // Self-reference check
                if parent_id == node.id {
                    return Err(AiomeError::Infrastructure {
                        reason: format!("Circular dependency: step {} refers to itself", node.id),
                    });
                }

                // Verify parent exists
                if !id_map.contains_key(&parent_id) {
                    continue; // Or handle as orphan
                }

                edges.push(GraphEdge {
                    from: parent_id,
                    to: node.id,
                });
            }
        }

        // 3. Simple Circularity Check (A -> B -> A)
        // For a true DAG check, we can use a small DFS, but given the time-ordered nature of jobs,
        // we mainly want to prevent obvious cycles that crash the UI.
        let mut depth_map: HashMap<u32, usize> = HashMap::new();
        for node in &nodes {
            let mut current = node.id;
            let mut path = HashSet::new();
            let mut depth = 0;

            while let Some(parent) = id_map.get(&current).and_then(|s| s.parent_step_id) {
                if path.contains(&parent) {
                    return Err(AiomeError::Infrastructure {
                        reason: format!("Circular dependency detected involving step {}", parent),
                    });
                }
                path.insert(parent);

                if let Some(&d) = depth_map.get(&parent) {
                    depth += d + 1;
                    break;
                }

                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(AiomeError::Infrastructure {
                        reason: format!(
                            "Trajectory depth exceeded: {} levels exceeds limit of {}",
                            depth, MAX_DEPTH
                        ),
                    });
                }
                current = parent;
            }
            depth_map.insert(node.id, depth);
            if depth > MAX_DEPTH {
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "Trajectory depth exceeded at leaf: {} levels exceeds limit of {}",
                        depth, MAX_DEPTH
                    ),
                });
            }
        }

        Ok(Self { nodes, edges })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::trajectory::StepCategory;

    fn mock_step(id: u32, parent: Option<u32>) -> TrajectoryStep {
        TrajectoryStep {
            step_id: id,
            parent_step_id: parent,
            action: format!("Step {}", id),
            step_category: StepCategory::Execution,
            ..Default::default()
        }
    }

    #[test]
    fn test_build_graph_linear() {
        let steps = vec![
            mock_step(1, None),
            mock_step(2, Some(1)),
            mock_step(3, Some(2)),
        ];

        let graph = TrajectoryGraph::build_graph(steps).expect("Should build graph"); // allow-anti-pattern
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);

        assert!(graph.edges.iter().any(|e| e.from == 1 && e.to == 2));
        assert!(graph.edges.iter().any(|e| e.from == 2 && e.to == 3));
    }

    #[test]
    fn test_build_graph_circular_protection() {
        let steps = vec![mock_step(1, Some(2)), mock_step(2, Some(1))];

        let result = TrajectoryGraph::build_graph(steps);
        assert!(result.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = result {
            assert!(reason.contains("Circular") || reason.contains("loop"));
        } else {
            panic!("Expected circular error");
        }
    }

    #[test]
    fn test_build_graph_limits() {
        // 1. Exceed MAX_NODES (1000)
        let mut many_steps = Vec::new();
        for i in 1..=1001 {
            many_steps.push(mock_step(i, None));
        }
        let res_nodes = TrajectoryGraph::build_graph(many_steps);
        assert!(res_nodes.is_err());
        assert!(format!("{:?}", res_nodes).contains("too large"));

        // 2. Exceed MAX_DEPTH (50)
        let mut deep_steps = Vec::new();
        for i in 1..=55 {
            deep_steps.push(mock_step(i, if i == 1 { None } else { Some(i - 1) }));
        }
        let res_depth = TrajectoryGraph::build_graph(deep_steps);
        assert!(res_depth.is_err());
        assert!(format!("{:?}", res_depth).contains("depth exceeded"));
    }
}
