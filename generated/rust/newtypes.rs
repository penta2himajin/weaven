#[allow(unused_imports)]
use crate::models::*;
#[allow(unused_imports)]
use crate::fixtures::*;
#[allow(unused_imports)]
use crate::helpers::*;

/// Newtype wrapper: TickCursor validated by CursorRange.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedTickCursor(pub TickCursor);

impl TryFrom<TickCursor> for ValidatedTickCursor {
    type Error = &'static str;

    fn try_from(value: TickCursor) -> Result<Self, Self::Error> {
        if value.current > value.maxTick {
            return Err("current must be <= maxTick");
        }
        let tick_cursors: Vec<TickCursor> = vec![value.clone()];
        if tick_cursors.iter().all(|c| { let c = c.clone(); c.current >= 0 && c.current <= c.maxTick }) {
            Ok(ValidatedTickCursor(value))
        } else {
            Err("CursorRange invariant violated")
        }
    }
}

/// Newtype wrapper: TopologyGraph validated by EdgesReferenceGraphNodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedTopologyGraph(pub TopologyGraph);

impl TryFrom<TopologyGraph> for ValidatedTopologyGraph {
    type Error = &'static str;

    fn try_from(value: TopologyGraph) -> Result<Self, Self::Error> {
        let topology_graphs: Vec<TopologyGraph> = vec![value.clone()];
        if topology_graphs.iter().all(|g| { let g = g.clone(); g.edges.iter().all(|e| { let e = e.clone(); g.nodes.contains(&e.edgeSource) && g.nodes.contains(&e.edgeTarget) }) }) {
            Ok(ValidatedTopologyGraph(value))
        } else {
            Err("EdgesReferenceGraphNodes invariant violated")
        }
    }
}

/// Newtype wrapper: EvalTreeNode validated by NoCyclicEvalTree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedEvalTreeNode(pub EvalTreeNode);

impl TryFrom<EvalTreeNode> for ValidatedEvalTreeNode {
    type Error = &'static str;

    fn try_from(value: EvalTreeNode) -> Result<Self, Self::Error> {
        let eval_tree_nodes: Vec<EvalTreeNode> = vec![value.clone()];
        if !eval_tree_nodes.iter().any(|n| { let n = n.clone(); tc_children(&n).contains(&n) }) {
            Ok(ValidatedEvalTreeNode(value))
        } else {
            Err("NoCyclicEvalTree invariant violated")
        }
    }
}

/// Newtype wrapper: GraphEdge validated by NoSelfLoop.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedGraphEdge(pub GraphEdge);

impl TryFrom<GraphEdge> for ValidatedGraphEdge {
    type Error = &'static str;

    fn try_from(value: GraphEdge) -> Result<Self, Self::Error> {
        let graph_edges: Vec<GraphEdge> = vec![value.clone()];
        if !graph_edges.iter().any(|e| { let e = e.clone(); e.edgeSource == e.edgeTarget }) {
            Ok(ValidatedGraphEdge(value))
        } else {
            Err("NoSelfLoop invariant violated")
        }
    }
}

/// Newtype wrapper: DebugSession validated by SnapshotNonEmpty.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedDebugSession(pub DebugSession);

impl TryFrom<DebugSession> for ValidatedDebugSession {
    type Error = &'static str;

    fn try_from(value: DebugSession) -> Result<Self, Self::Error> {
        if value.snapshots.len() < 1 {
            return Err("SnapshotNonEmpty: snapshots has fewer than 1 elements");
        }
        let debug_sessions: Vec<DebugSession> = vec![value.clone()];
        if debug_sessions.iter().all(|d| { let d = d.clone(); d.snapshots.len() > 0 }) {
            Ok(ValidatedDebugSession(value))
        } else {
            Err("SnapshotNonEmpty invariant violated")
        }
    }
}

