use std::collections::{BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InteractionRuleId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    PhaseInput,
    PhaseEvaluate,
    PhaseExecute,
    PhasePropagate,
    PhaseLifecycle,
    PhaseOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceEvent {
    GuardEvaluated {
        tick: Tick,
        phase: Phase,
        transition: TransitionId,
        smId: SmId,
        result: Bool,
    },
    IrMatched {
        tick: Tick,
        phase: Phase,
        ruleId: InteractionRuleId,
        participants: BTreeSet<SmId>,
    },
    TransitionFired {
        tick: Tick,
        phase: Phase,
        transition: TransitionId,
        smId: SmId,
        fromState: StateId,
        toState: StateId,
    },
    SignalEmitted {
        tick: Tick,
        phase: Phase,
        smId: SmId,
        port: PortId,
        target: Option<SmId>,
    },
    CascadeStep {
        tick: Tick,
        phase: Phase,
        depth: Int,
        queueSize: Int,
    },
    PipelineFiltered {
        tick: Tick,
        phase: Phase,
        connection: Option<ConnectionId>,
        smId: SmId,
        port: PortId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    EdgeStatic,
    EdgeSpatial,
    EdgeIR,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphNode {
    pub sm: SmId,
    pub activeState: Option<StateId>,
}

/// Invariant: NoSelfLoop
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphEdge {
    pub edgeSource: GraphNode,
    pub edgeTarget: GraphNode,
    pub kind: EdgeKind,
    pub connectionId: Option<ConnectionId>,
}

/// Invariant: EdgesReferenceGraphNodes
/// Invariant: UniqueSmPerNode
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopologyGraph {
    pub nodes: BTreeSet<GraphNode>,
    pub edges: BTreeSet<GraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExprKind {
    ExprLit,
    ExprCtxRef,
    ExprSigRef,
    ExprTableRef,
    ExprBinOp,
    ExprNotOp,
    ExprIfOp,
    ExprPortRecv,
}

/// Invariant: NoCyclicEvalTree
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvalTreeNode {
    pub exprKind: ExprKind,
    pub label: Label,
    pub value: EvalValue,
    pub children: Vec<EvalTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvalValue;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextSnapshot {
    pub fields: BTreeSet<ContextEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextEntry {
    pub fieldName: Label,
    pub fieldValue: EvalValue,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignalSnapshot {
    pub signalFields: BTreeSet<ContextEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardInspectionResult {
    pub transition: TransitionId,
    pub fired: Bool,
    pub contextAtEval: ContextSnapshot,
    pub signalAtEval: Option<SignalSnapshot>,
    pub exprTree: EvalTreeNode,
}

/// Invariant: CursorRange
/// Invariant: MaxTickNonNeg
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickCursor {
    pub current: Int,
    pub maxTick: Int,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldSnapshot;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilterConfig {
    pub visibleSms: BTreeSet<SmId>,
    pub visibleConnections: BTreeSet<ConnectionId>,
    pub visiblePhases: BTreeSet<Phase>,
}

/// Invariant: SnapshotNonEmpty
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DebugSession {
    pub snapshots: Vec<WorldSnapshot>,
    pub cursor: TickCursor,
    pub selectedSm: Option<SmId>,
    pub trace: Vec<TraceEvent>,
    pub topology: TopologyGraph,
    pub filterCfg: FilterConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickResult {
    pub traceEvents: Vec<TraceEvent>,
    pub stateChanges: BTreeSet<StateChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateChange {
    pub smId: SmId,
    pub fromState: StateId,
    pub toState: StateId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldState {
    pub smStates: BTreeSet<SmStateEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmStateEntry {
    pub smId: SmId,
    pub activeState: StateId,
}

