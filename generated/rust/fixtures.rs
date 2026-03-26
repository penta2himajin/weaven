#[allow(unused_imports)]
use crate::models::*;
#[allow(unused_imports)]
use std::collections::BTreeSet;

/// Factory: default value for enum Phase
#[allow(dead_code)]
pub fn default_phase() -> Phase {
    Phase::PhaseInput
}

/// Factory: default value for enum EdgeKind
#[allow(dead_code)]
pub fn default_edge_kind() -> EdgeKind {
    EdgeKind::EdgeStatic
}

/// Factory: default value for enum ExprKind
#[allow(dead_code)]
pub fn default_expr_kind() -> ExprKind {
    ExprKind::ExprLit
}

/// Factory: default value for unit struct SmId
#[allow(dead_code)]
pub fn default_sm_id() -> SmId { SmId }

/// Factory: default value for unit struct StateId
#[allow(dead_code)]
pub fn default_state_id() -> StateId { StateId }

/// Factory: default value for unit struct TransitionId
#[allow(dead_code)]
pub fn default_transition_id() -> TransitionId { TransitionId }

/// Factory: default value for unit struct PortId
#[allow(dead_code)]
pub fn default_port_id() -> PortId { PortId }

/// Factory: default value for unit struct ConnectionId
#[allow(dead_code)]
pub fn default_connection_id() -> ConnectionId { ConnectionId }

/// Factory: default value for unit struct InteractionRuleId
#[allow(dead_code)]
pub fn default_interaction_rule_id() -> InteractionRuleId { InteractionRuleId }

/// Factory: default value for unit struct Tick
#[allow(dead_code)]
pub fn default_tick() -> Tick { Tick }

/// Factory: create a default valid GraphNode
#[allow(dead_code)]
pub fn default_graph_node() -> GraphNode {
    GraphNode {
        sm: default_sm_id(),
        activeState: None,
    }
}

/// Factory: create a default valid GraphEdge
#[allow(dead_code)]
pub fn default_graph_edge() -> GraphEdge {
    GraphEdge {
        edgeSource: default_graph_node(),
        edgeTarget: default_graph_node(),
        kind: default_edge_kind(),
        connectionId: None,
    }
}

/// Factory: create a default valid TopologyGraph
#[allow(dead_code)]
pub fn default_topology_graph() -> TopologyGraph {
    TopologyGraph {
        nodes: BTreeSet::from([default_graph_node()]),
        edges: BTreeSet::from([default_graph_edge()]),
    }
}

/// Factory: create a default valid EvalTreeNode
#[allow(dead_code)]
pub fn default_eval_tree_node() -> EvalTreeNode {
    EvalTreeNode {
        exprKind: default_expr_kind(),
        label: default_label(),
        value: default_eval_value(),
        children: Vec::new(),
    }
}

/// Factory: default value for unit struct Label
#[allow(dead_code)]
pub fn default_label() -> Label { Label }

/// Factory: default value for unit struct EvalValue
#[allow(dead_code)]
pub fn default_eval_value() -> EvalValue { EvalValue }

/// Factory: create a default valid ContextSnapshot
#[allow(dead_code)]
pub fn default_context_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        fields: BTreeSet::from([default_context_entry()]),
    }
}

/// Factory: create a default valid ContextEntry
#[allow(dead_code)]
pub fn default_context_entry() -> ContextEntry {
    ContextEntry {
        fieldName: default_label(),
        fieldValue: default_eval_value(),
    }
}

/// Factory: create a default valid SignalSnapshot
#[allow(dead_code)]
pub fn default_signal_snapshot() -> SignalSnapshot {
    SignalSnapshot {
        signalFields: BTreeSet::from([default_context_entry()]),
    }
}

/// Factory: create a default valid GuardInspectionResult
#[allow(dead_code)]
pub fn default_guard_inspection_result() -> GuardInspectionResult {
    GuardInspectionResult {
        transition: default_transition_id(),
        fired: default_bool(),
        contextAtEval: default_context_snapshot(),
        signalAtEval: None,
        exprTree: default_eval_tree_node(),
    }
}

/// Factory: create a default valid TickCursor
#[allow(dead_code)]
pub fn default_tick_cursor() -> TickCursor {
    TickCursor {
        current: default_int(),
        maxTick: default_int(),
    }
}

/// Factory: default value for unit struct WorldSnapshot
#[allow(dead_code)]
pub fn default_world_snapshot() -> WorldSnapshot { WorldSnapshot }

/// Factory: create a default valid FilterConfig
#[allow(dead_code)]
pub fn default_filter_config() -> FilterConfig {
    FilterConfig {
        visibleSms: BTreeSet::new(),
        visibleConnections: BTreeSet::new(),
        visiblePhases: BTreeSet::new(),
    }
}

/// Factory: create a default valid DebugSession
#[allow(dead_code)]
pub fn default_debug_session() -> DebugSession {
    DebugSession {
        snapshots: Vec::new(),
        cursor: default_tick_cursor(),
        selectedSm: None,
        trace: Vec::new(),
        topology: default_topology_graph(),
        filterCfg: default_filter_config(),
    }
}

/// Factory: create DebugSession at cardinality boundary
#[allow(dead_code)]
pub fn boundary_debug_session() -> DebugSession {
    DebugSession {
        snapshots: vec![default_world_snapshot()],
        cursor: default_tick_cursor(),
        selectedSm: None,
        trace: Vec::new(),
        topology: default_topology_graph(),
        filterCfg: default_filter_config(),
    }
}

/// Factory: create DebugSession that violates cardinality constraint
#[allow(dead_code)]
pub fn invalid_debug_session() -> DebugSession {
    DebugSession {
        snapshots: Vec::new(),
        cursor: default_tick_cursor(),
        selectedSm: None,
        trace: Vec::new(),
        topology: default_topology_graph(),
        filterCfg: default_filter_config(),
    }
}

/// Factory: create a default valid TickResult
#[allow(dead_code)]
pub fn default_tick_result() -> TickResult {
    TickResult {
        traceEvents: Vec::new(),
        stateChanges: BTreeSet::from([default_state_change()]),
    }
}

/// Factory: create a default valid StateChange
#[allow(dead_code)]
pub fn default_state_change() -> StateChange {
    StateChange {
        smId: default_sm_id(),
        fromState: default_state_id(),
        toState: default_state_id(),
    }
}

/// Factory: create a default valid WorldState
#[allow(dead_code)]
pub fn default_world_state() -> WorldState {
    WorldState {
        smStates: BTreeSet::from([default_sm_state_entry()]),
    }
}

/// Factory: create a default valid SmStateEntry
#[allow(dead_code)]
pub fn default_sm_state_entry() -> SmStateEntry {
    SmStateEntry {
        smId: default_sm_id(),
        activeState: default_state_id(),
    }
}

