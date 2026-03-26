#[cfg(test)]
mod property_tests {
    #[allow(unused_imports)]
    use crate::models::*;
    #[allow(unused_imports)]
    use crate::helpers::*;
    #[allow(unused_imports)]
    use crate::fixtures::*;

    #[test]
    fn invariant_no_self_loop() {
        let graph_edges: Vec<GraphEdge> = vec![default_graph_edge()];
        assert!(!graph_edges.iter().any(|e| { let e = e.clone(); e.edgeSource == e.edgeTarget }));
    }

    #[test]
    fn invariant_edges_reference_graph_nodes() {
        let topology_graphs: Vec<TopologyGraph> = vec![default_topology_graph()];
        assert!(topology_graphs.iter().all(|g| { let g = g.clone(); g.edges.iter().all(|e| { let e = e.clone(); g.nodes.contains(&e.edgeSource) && g.nodes.contains(&e.edgeTarget) }) }));
    }

    #[test]
    fn invariant_unique_sm_per_node() {
        let topology_graphs: Vec<TopologyGraph> = vec![default_topology_graph()];
        assert!(topology_graphs.iter().all(|g| { let g = g.clone(); g.nodes.iter().all(|n1| { let n1 = n1.clone(); g.nodes.iter().all(|n2| { let n2 = n2.clone(); if n1 != n2 { n1.sm != n2.sm } else { true } }) }) }));
    }

    #[test]
    fn invariant_no_cyclic_eval_tree() {
        let eval_tree_nodes: Vec<EvalTreeNode> = vec![default_eval_tree_node()];
        assert!(!eval_tree_nodes.iter().any(|n| { let n = n.clone(); tc_children(&n).contains(&n) }));
    }

    #[test]
    fn invariant_cursor_range() {
        let tick_cursors: Vec<TickCursor> = vec![default_tick_cursor()];
        assert!(tick_cursors.iter().all(|c| { let c = c.clone(); c.current >= 0 && c.current <= c.maxTick }));
    }

    #[test]
    fn invariant_max_tick_non_neg() {
        let tick_cursors: Vec<TickCursor> = vec![default_tick_cursor()];
        assert!(tick_cursors.iter().all(|c| { let c = c.clone(); c.maxTick >= 0 }));
    }

    /// @regression Partially type-guaranteed — regression test only.
    #[test]
    fn invariant_snapshot_non_empty() {
        let debug_sessions: Vec<DebugSession> = vec![default_debug_session()];
        assert!(debug_sessions.iter().all(|d| { let d = d.clone(); d.snapshots.len() > 0 }));
    }

    #[test]
    fn boundary_snapshot_non_empty() {
        let debug_sessions: Vec<DebugSession> = vec![boundary_debug_session()];
        assert!(debug_sessions.iter().all(|d| { let d = d.clone(); d.snapshots.len() > 0 }), "boundary values should satisfy invariant");
    }

    #[test]
    fn invalid_snapshot_non_empty() {
        let debug_sessions: Vec<DebugSession> = vec![invalid_debug_session()];
        assert!(!(debug_sessions.iter().all(|d| { let d = d.clone(); d.snapshots.len() > 0 })), "invalid values should violate invariant");
    }

}
