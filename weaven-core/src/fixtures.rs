#[allow(unused_imports)]
use crate::models::*;
#[allow(unused_imports)]
use std::collections::BTreeSet;

/// Factory: default value for enum PortKind
#[allow(dead_code)]
pub fn default_port_kind() -> PortKind {
    PortKind::PortKindInput
}

/// Factory: default value for enum SuspendPolicy
#[allow(dead_code)]
pub fn default_suspend_policy() -> SuspendPolicy {
    SuspendPolicy::Freeze
}

/// Factory: default value for enum ElapseCapability
#[allow(dead_code)]
pub fn default_elapse_capability() -> ElapseCapability {
    ElapseCapability::Deterministic
}

/// Factory: default value for enum Authority
#[allow(dead_code)]
pub fn default_authority() -> Authority {
    Authority::AuthServer
}

/// Factory: default value for enum SyncPolicy
#[allow(dead_code)]
pub fn default_sync_policy() -> SyncPolicy {
    SyncPolicy::InputSync
}

/// Factory: default value for enum Reconciliation
#[allow(dead_code)]
pub fn default_reconciliation() -> Reconciliation {
    Reconciliation::Snap
}

/// Factory: default value for enum PipelineStepKind
#[allow(dead_code)]
pub fn default_pipeline_step_kind() -> PipelineStepKind {
    PipelineStepKind::StepTransform
}

/// Factory: default value for enum ExprNode
#[allow(dead_code)]
pub fn default_expr_node() -> ExprNode {
    ExprNode::ExprLiteral
}

/// Factory: default value for enum BinOp
#[allow(dead_code)]
pub fn default_bin_op() -> BinOp {
    BinOp::OpAdd
}

/// Factory: default value for unit struct SignalType
#[allow(dead_code)]
pub fn default_signal_type() -> SignalType { SignalType }

/// Factory: create a default valid Signal
#[allow(dead_code)]
pub fn default_signal() -> Signal {
    Signal {
        signalType: default_signal_type(),
    }
}

/// Factory: create a default valid PipelineStep
#[allow(dead_code)]
pub fn default_pipeline_step() -> PipelineStep {
    PipelineStep {
        stepKind: default_pipeline_step_kind(),
        stepTarget: None,
    }
}

/// Factory: create a default valid Pipeline
#[allow(dead_code)]
pub fn default_pipeline() -> Pipeline {
    Pipeline {
        steps: vec![default_pipeline_step()],
    }
}

/// Factory: create a default valid Port
#[allow(dead_code)]
pub fn default_port() -> Port {
    Port {
        portKind: default_port_kind(),
        signalType: default_signal_type(),
        pipeline: None,
    }
}

/// Factory: create a default valid State
#[allow(dead_code)]
pub fn default_state() -> State {
    State {
        contextFields: BTreeSet::new(),
    }
}

/// Factory: default value for unit struct ContextField
#[allow(dead_code)]
pub fn default_context_field() -> ContextField { ContextField }

/// Factory: default value for unit struct Guard
#[allow(dead_code)]
pub fn default_guard() -> Guard { Guard }

/// Factory: default value for unit struct Effect
#[allow(dead_code)]
pub fn default_effect() -> Effect { Effect }

/// Factory: create a default valid Transition
#[allow(dead_code)]
pub fn default_transition() -> Transition {
    Transition {
        source: default_state(),
        target: default_state(),
        priority: default_priority(),
        guard: None,
        effects: BTreeSet::new(),
    }
}

/// Factory: default value for unit struct Priority
#[allow(dead_code)]
pub fn default_priority() -> Priority { Priority }

/// Factory: create a default valid StateMachine
#[allow(dead_code)]
pub fn default_state_machine() -> StateMachine {
    StateMachine {
        states: BTreeSet::from([default_state()]),
        transitions: BTreeSet::from([default_transition()]),
        ports: BTreeSet::from([default_port()]),
        initialState: default_state(),
        activeState: None,
        elapseCapability: default_elapse_capability(),
        authority: default_authority(),
        syncPolicy: default_sync_policy(),
        reconciliation: default_reconciliation(),
    }
}

/// Factory: create a default valid Connection
#[allow(dead_code)]
pub fn default_connection() -> Connection {
    Connection {
        connSource: default_port(),
        connTarget: default_port(),
        connPipeline: None,
        delayTicks: default_delay_ticks(),
    }
}

/// Factory: default value for unit struct DelayTicks
#[allow(dead_code)]
pub fn default_delay_ticks() -> DelayTicks { DelayTicks }

/// Factory: create a default valid IRGroup
#[allow(dead_code)]
pub fn default_i_r_group() -> IRGroup {
    IRGroup {
        groupName: default_i_r_group_name(),
    }
}

/// Factory: default value for unit struct IRGroupName
#[allow(dead_code)]
pub fn default_i_r_group_name() -> IRGroupName { IRGroupName }

/// Factory: create a default valid IRParticipant
#[allow(dead_code)]
pub fn default_i_r_participant() -> IRParticipant {
    IRParticipant {
        requiredPortKind: default_port_kind(),
        requiredSignalType: default_signal_type(),
    }
}

/// Factory: create a default valid IRResult
#[allow(dead_code)]
pub fn default_i_r_result() -> IRResult {
    IRResult {
        resultParticipant: default_i_r_participant(),
        resultPort: default_port(),
        resultSignalType: default_signal_type(),
    }
}

/// Factory: create a default valid InteractionRule
#[allow(dead_code)]
pub fn default_interaction_rule() -> InteractionRule {
    InteractionRule {
        participants: BTreeSet::from([default_i_r_participant()]),
        results: BTreeSet::from([default_i_r_result()]),
        group: default_i_r_group(),
    }
}

/// Factory: create InteractionRule at cardinality boundary
#[allow(dead_code)]
pub fn boundary_interaction_rule() -> InteractionRule {
    InteractionRule {
        participants: BTreeSet::from([default_i_r_participant(), default_i_r_participant()]),
        results: BTreeSet::from([default_i_r_result()]),
        group: default_i_r_group(),
    }
}

/// Factory: create InteractionRule that violates cardinality constraint
#[allow(dead_code)]
pub fn invalid_interaction_rule() -> InteractionRule {
    InteractionRule {
        participants: BTreeSet::from([default_i_r_participant()]),
        results: BTreeSet::new(),
        group: default_i_r_group(),
    }
}

/// Factory: default value for unit struct TableValue
#[allow(dead_code)]
pub fn default_table_value() -> TableValue { TableValue }

/// Factory: create a default valid TableEntry
#[allow(dead_code)]
pub fn default_table_entry() -> TableEntry {
    TableEntry {
        entryKey: default_table_key(),
        entryValue: default_table_value(),
    }
}

/// Factory: default value for unit struct TableKey
#[allow(dead_code)]
pub fn default_table_key() -> TableKey { TableKey }

/// Factory: create a default valid NamedTable
#[allow(dead_code)]
pub fn default_named_table() -> NamedTable {
    NamedTable {
        tableName: default_table_key(),
        tableEntries: BTreeSet::from([default_table_entry()]),
    }
}

/// Factory: create a default valid Entity
#[allow(dead_code)]
pub fn default_entity() -> Entity {
    Entity {
        machines: BTreeSet::from([default_state_machine()]),
    }
}

/// Factory: create a default valid CompoundState
#[allow(dead_code)]
pub fn default_compound_state() -> CompoundState {
    CompoundState {
        compoundParent: default_state(),
        subMachines: BTreeSet::from([default_state_machine()]),
        suspendPolicy: default_suspend_policy(),
        promotions: BTreeSet::new(),
    }
}

/// Factory: create a default valid PortPromotion
#[allow(dead_code)]
pub fn default_port_promotion() -> PortPromotion {
    PortPromotion {
        promotedPort: Box::new(default_port()),
        promotionOwner: Box::new(default_compound_state()),
    }
}

/// Factory: create a default valid ActiveSet
#[allow(dead_code)]
pub fn default_active_set() -> ActiveSet {
    ActiveSet {
        activeMachines: BTreeSet::from([default_state_machine()]),
    }
}

