//! TopologyGraph — builds the visual graph structure from a World.
//!
//! Design doc §5.2: GraphNode (SM), GraphEdge (Connection/IR), EdgeKind.

use serde::Serialize;
use weaven_core::{SmId, StateId, ConnectionId, World};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum EdgeKind {
    Static,
    Spatial,
    IR,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub sm_id: SmId,
    pub active_state: Option<StateId>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: SmId,
    pub target: SmId,
    pub kind: EdgeKind,
    pub connection_id: Option<ConnectionId>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Build a TopologyGraph from the current World state.
pub fn build_topology(world: &World) -> TopologyGraph {
    // Nodes: one per registered SM.
    let nodes: Vec<GraphNode> = world.defs.keys().map(|&sm_id| {
        let active_state = world.instances.get(&sm_id).map(|i| i.active_state);
        let label = format!("SM({})", sm_id.0);
        GraphNode { sm_id, active_state, label }
    }).collect();

    // Edges from static Connections.
    let mut edges: Vec<GraphEdge> = world.connections.iter().map(|conn| {
        let pipeline_len = conn.pipeline.len();
        let label = if pipeline_len > 0 {
            format!("C({}) [{}p]", conn.id.0, pipeline_len)
        } else {
            format!("C({})", conn.id.0)
        };
        GraphEdge {
            source: conn.source_sm,
            target: conn.target_sm,
            kind: EdgeKind::Static,
            connection_id: Some(conn.id),
            label,
        }
    }).collect();

    // Edges from InteractionRules (represented as IR edges).
    // IRs don't have fixed source/target; we represent them as
    // edges between all participant SMs detected in the last evaluation.
    // For the static topology view, we show the rule's existence
    // by connecting all SMs that have matching port types.
    for (idx, _rule) in world.interaction_rules.iter().enumerate() {
        // In a full implementation, we'd inspect the rule's match_fn
        // to determine participant SMs. For now, IR edges are added
        // dynamically when trace events reveal IrMatched participants.
        let _ = idx;
    }

    TopologyGraph { nodes, edges }
}

/// Augment topology with IR edges from trace events.
pub fn add_ir_edges_from_trace(
    graph: &mut TopologyGraph,
    trace_events: &[weaven_core::trace::TraceEvent],
) {
    use weaven_core::trace::TraceEvent;

    for event in trace_events {
        if let TraceEvent::IrMatched { rule_index, participants, .. } = event {
            // Create edges between all participant pairs.
            for (i, &sm_a) in participants.iter().enumerate() {
                for &sm_b in &participants[i + 1..] {
                    let already_exists = graph.edges.iter().any(|e| {
                        e.kind == EdgeKind::IR
                            && ((e.source == sm_a && e.target == sm_b)
                                || (e.source == sm_b && e.target == sm_a))
                    });
                    if !already_exists {
                        graph.edges.push(GraphEdge {
                            source: sm_a,
                            target: sm_b,
                            kind: EdgeKind::IR,
                            connection_id: None,
                            label: format!("IR({})", rule_index),
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weaven_core::*;

    #[test]
    fn topology_from_empty_world() {
        let world = World::new();
        let graph = build_topology(&world);
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn topology_nodes_match_sms() {
        let mut world = World::new();
        world.register_sm(SmDef {
            id: SmId(1),
            states: [StateId(0)].into_iter().collect(),
            initial_state: StateId(0),
            transitions: vec![],
            input_ports: vec![],
            output_ports: vec![],
            on_despawn_transitions: vec![],
            elapse_capability: ElapseCapabilityRt::NonElapsable,
            elapse_fn: None,
        });
        world.register_sm(SmDef {
            id: SmId(2),
            states: [StateId(0)].into_iter().collect(),
            initial_state: StateId(0),
            transitions: vec![],
            input_ports: vec![],
            output_ports: vec![],
            on_despawn_transitions: vec![],
            elapse_capability: ElapseCapabilityRt::NonElapsable,
            elapse_fn: None,
        });

        let graph = build_topology(&world);
        assert_eq!(graph.nodes.len(), 2);
    }

    #[test]
    fn topology_edges_from_connections() {
        let mut world = World::new();
        for i in 1..=2 {
            world.register_sm(SmDef {
                id: SmId(i),
                states: [StateId(0)].into_iter().collect(),
                initial_state: StateId(0),
                transitions: vec![],
                input_ports: vec![Port::new(PortId(0), PortKind::Input, SignalTypeId(0))],
                output_ports: vec![Port::new(PortId(1), PortKind::Output, SignalTypeId(0))],
                on_despawn_transitions: vec![],
                elapse_capability: ElapseCapabilityRt::NonElapsable,
                elapse_fn: None,
            });
        }
        world.connections.push(Connection {
            id: ConnectionId(1),
            source_sm: SmId(1),
            source_port: PortId(1),
            target_sm: SmId(2),
            target_port: PortId(0),
            delay_ticks: 0,
            pipeline: vec![],
        });

        let graph = build_topology(&world);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].kind, EdgeKind::Static);
        assert_eq!(graph.edges[0].source, SmId(1));
        assert_eq!(graph.edges[0].target, SmId(2));
    }
}
