use std::collections::{BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PortKind {
    PortKindInput,
    PortKindOutput,
    PortKindContinuousInput,
    PortKindContinuousOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SuspendPolicy {
    Freeze,
    Elapse,
    Discard,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElapseCapability {
    Deterministic,
    Approximate,
    NonElapsable,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Authority {
    AuthServer,
    AuthOwner,
    AuthLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyncPolicy {
    InputSync,
    StateSync,
    ContextSync,
    NoSync,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reconciliation {
    Snap,
    Interpolate,
    Rewind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PipelineStepKind {
    StepTransform,
    StepFilter,
    StepRedirect,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignalType;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signal {
    pub signalType: SignalType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PipelineStep {
    pub stepKind: PipelineStepKind,
    pub stepTarget: Option<Box<Port>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pipeline {
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Port {
    pub portKind: PortKind,
    pub signalType: SignalType,
    pub pipeline: Option<Box<Pipeline>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State {
    pub contextFields: BTreeSet<ContextField>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextField;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guard;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Effect;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transition {
    pub source: State,
    pub target: State,
    pub priority: Priority,
    pub guard: Option<Guard>,
    pub effects: BTreeSet<Effect>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Priority;

/// Invariant: InitialStateOwned
/// Invariant: ActiveStateOwned
/// Invariant: TransitionStatesOwned
/// Invariant: UniqueTransitionPriority
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateMachine {
    pub states: BTreeSet<State>,
    pub transitions: BTreeSet<Transition>,
    pub ports: BTreeSet<Port>,
    pub initialState: State,
    pub activeState: Option<State>,
    pub elapseCapability: ElapseCapability,
    pub authority: Authority,
    pub syncPolicy: SyncPolicy,
    pub reconciliation: Reconciliation,
}

/// Invariant: ConnectionDirectionality
/// Invariant: ConnectionSignalTypeCompat
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Connection {
    pub connSource: Port,
    pub connTarget: Port,
    pub connPipeline: Option<Pipeline>,
    pub delayTicks: DelayTicks,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DelayTicks;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IRGroup {
    pub groupName: IRGroupName,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IRGroupName;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IRParticipant {
    pub requiredPortKind: PortKind,
    pub requiredSignalType: SignalType,
}

/// Invariant: IRResultSignalTypeConsistent
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IRResult {
    pub resultParticipant: IRParticipant,
    pub resultPort: Port,
    pub resultSignalType: SignalType,
}

/// Invariant: InteractionRuleRequiresTwoParticipants
/// Invariant: IRResultTargetsOwnParticipant
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InteractionRule {
    pub participants: BTreeSet<IRParticipant>,
    pub results: BTreeSet<IRResult>,
    pub group: IRGroup,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableValue;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableEntry {
    pub entryKey: TableKey,
    pub entryValue: TableValue,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableKey;

/// Invariant: NamedTableNamesUnique
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamedTable {
    pub tableName: TableKey,
    pub tableEntries: BTreeSet<TableEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExprNode {
    ExprLiteral,
    ExprCtxField {
        ctxFieldName: TableKey,
    },
    ExprSigField {
        sigFieldName: TableKey,
    },
    ExprTableRef {
        tableRef: NamedTable,
        tableKey: Box<ExprNode>,
    },
    ExprBinOp {
        binOp: BinOp,
        binLeft: Box<ExprNode>,
        binRight: Box<ExprNode>,
    },
    ExprNot {
        notInner: Box<ExprNode>,
    },
    ExprIf {
        ifCond: Box<ExprNode>,
        ifThen: Box<ExprNode>,
        ifElse: Box<ExprNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinOp {
    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
    OpEq,
    OpNeq,
    OpLt,
    OpGt,
    OpLte,
    OpGte,
    OpAnd,
    OpOr,
}

/// Invariant: EntityHasMachines
/// Invariant: ActiveSetSubsetOfEntities
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    pub machines: BTreeSet<StateMachine>,
}

/// Invariant: SubMachineDoesNotOwnParentState
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompoundState {
    pub compoundParent: State,
    pub subMachines: BTreeSet<StateMachine>,
    pub suspendPolicy: SuspendPolicy,
    pub promotions: BTreeSet<PortPromotion>,
}

/// Invariant: PortPromotionOwnership
/// Invariant: PortPromotionOutputOnly
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortPromotion {
    pub promotedPort: Box<Port>,
    pub promotionOwner: Box<CompoundState>,
}

/// Invariant: ActiveSetSubsetOfEntities
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActiveSet {
    pub activeMachines: BTreeSet<StateMachine>,
}

