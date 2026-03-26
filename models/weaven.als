-- weaven.als
-- Formal specification of the Weaven Interaction-Topology-Oriented Game Framework.
-- Single source of truth for all Weaven primitive structures and invariants.
-- Processed by oxidtr to generate type definitions, validators, and tests.

-------------------------------------------------------------------------------
-- Enumerations
-------------------------------------------------------------------------------

abstract sig PortKind {}
one sig PortKindInput            extends PortKind {}
one sig PortKindOutput           extends PortKind {}
one sig PortKindContinuousInput  extends PortKind {}
one sig PortKindContinuousOutput extends PortKind {}

abstract sig SuspendPolicy {}
one sig Freeze  extends SuspendPolicy {}
one sig Elapse  extends SuspendPolicy {}
one sig Discard extends SuspendPolicy {}

abstract sig ElapseCapability {}
one sig Deterministic  extends ElapseCapability {}
one sig Approximate    extends ElapseCapability {}
one sig NonElapsable   extends ElapseCapability {}

abstract sig Authority {}
one sig AuthServer extends Authority {}
one sig AuthOwner  extends Authority {}
one sig AuthLocal  extends Authority {}

abstract sig SyncPolicy {}
one sig InputSync   extends SyncPolicy {}
one sig StateSync   extends SyncPolicy {}
one sig ContextSync extends SyncPolicy {}
one sig NoSync      extends SyncPolicy {}

abstract sig Reconciliation {}
one sig Snap        extends Reconciliation {}
one sig Interpolate extends Reconciliation {}
one sig Rewind      extends Reconciliation {}

abstract sig PipelineStepKind {}
one sig StepTransform extends PipelineStepKind {}
one sig StepFilter    extends PipelineStepKind {}
one sig StepRedirect  extends PipelineStepKind {}

-------------------------------------------------------------------------------
-- Signal type schema (named, typed)
-------------------------------------------------------------------------------

sig SignalType {}

sig Signal {
  signalType: one SignalType
}

-------------------------------------------------------------------------------
-- Pipeline
-------------------------------------------------------------------------------

sig PipelineStep {
  stepKind:   one PipelineStepKind,
  stepTarget: lone Port
}

sig Pipeline {
  steps: seq PipelineStep
}

-------------------------------------------------------------------------------
-- Ports
-------------------------------------------------------------------------------

sig Port {
  portKind:   one PortKind,
  signalType: one SignalType,
  pipeline:   lone Pipeline
}

-------------------------------------------------------------------------------
-- State and Transition
-------------------------------------------------------------------------------

sig State {
  contextFields: set ContextField
}

sig ContextField {}

-- Guard and Effect are represented abstractly.
-- The Expression Language (§5) is a separate subsystem.
sig Guard {}
sig Effect {}

sig Transition {
  source:   one State,
  target:   one State,
  priority: one Priority,
  guard:    lone Guard,
  effects:  set Effect
}

-- Priority is modeled as an abstract ordered token.
-- In the Rust implementation this maps to u32.
sig Priority {}

-------------------------------------------------------------------------------
-- StateMachine
-------------------------------------------------------------------------------

sig StateMachine {
  states:            set State,
  transitions:       set Transition,
  ports:             set Port,
  initialState:      one State,
  activeState:       lone State,
  elapseCapability:  one ElapseCapability,
  authority:         one Authority,
  syncPolicy:        one SyncPolicy,
  reconciliation:    one Reconciliation
}

-------------------------------------------------------------------------------
-- Connection
-------------------------------------------------------------------------------

sig Connection {
  connSource:   one Port,
  connTarget:   one Port,
  connPipeline: lone Pipeline,
  delayTicks:   one DelayTicks
}

-- Delay in ticks (non-negative). Mapped to u32 in Rust.
sig DelayTicks {}

-------------------------------------------------------------------------------
-- Interaction Rule
-------------------------------------------------------------------------------

-------------------------------------------------------------------------------
-- Interaction Rule (§2.7)
-------------------------------------------------------------------------------

-- Namespace for grouping related rules (e.g. "elemental_reactions", "combat").
sig IRGroup {
  groupName: one IRGroupName
}

-- Opaque name token (maps to a string in the runtime).
sig IRGroupName {}

-- Describes one participant side of an Interaction Rule match:
-- which Port kind and SignalType the target SM must expose.
sig IRParticipant {
  requiredPortKind:   one PortKind,
  requiredSignalType: one SignalType
}

-- Describes a signal to deliver when the rule matches.
sig IRResult {
  resultParticipant:  one IRParticipant,
  resultPort:         one Port,
  resultSignalType:   one SignalType
}

sig InteractionRule {
  participants: set IRParticipant,
  results:      set IRResult,
  group:        one IRGroup
}

-- An Interaction Rule must involve at least two participants (§2.7: multi-entity).
fact InteractionRuleRequiresTwoParticipants {
  all ir: InteractionRule | #ir.participants >= 2
}

-- Every result must target a participant of the same rule.
fact IRResultTargetsOwnParticipant {
  all ir: InteractionRule | all r: ir.results |
    r.resultParticipant in ir.participants
}

-- Result signal type must match the result port's declared signal type.
fact IRResultSignalTypeConsistent {
  all r: IRResult | r.resultSignalType = r.resultPort.signalType
}

-------------------------------------------------------------------------------
-- Named Table
-------------------------------------------------------------------------------

-------------------------------------------------------------------------------
-- Named Table (§2.8)
-------------------------------------------------------------------------------

-- A single value in a NamedTable (leaf node in the nested map).
sig TableValue {}

-- A keyed entry within a NamedTable (one level of nesting).
sig TableEntry {
  entryKey:   one TableKey,
  entryValue: one TableValue
}

-- Key token used for table lookup.
sig TableKey {}

-- Global, read-only, keyed data structure (§2.8).
-- Cannot be modified at runtime.
sig NamedTable {
  tableName:    one TableKey,
  tableEntries: set TableEntry
}

-- Named tables are globally unique by name.
fact NamedTableNamesUnique {
  all t1: NamedTable | all t2: NamedTable |
    t1 != t2 implies t1.tableName != t2.tableName
}

-------------------------------------------------------------------------------
-- Expression Language AST (§5)
-------------------------------------------------------------------------------

-- The Expression Language is a restricted declarative language.
-- Implemented as a tree-walking AST evaluated left-to-right (§5.3).

abstract sig ExprNode {}

-- Terminals
sig ExprLiteral  extends ExprNode {}           -- numeric / bool literal
sig ExprCtxField extends ExprNode {            -- context.<field>
  ctxFieldName: one TableKey
}
sig ExprSigField extends ExprNode {            -- signal.<field>
  sigFieldName: one TableKey
}
sig ExprTableRef extends ExprNode {            -- table.<name>[key]
  tableRef:     one NamedTable,
  tableKey:     one ExprNode
}

-- Operators
abstract sig BinOp {}
one sig OpAdd  extends BinOp {}
one sig OpSub  extends BinOp {}
one sig OpMul  extends BinOp {}
one sig OpDiv  extends BinOp {}
one sig OpEq   extends BinOp {}
one sig OpNeq  extends BinOp {}
one sig OpLt   extends BinOp {}
one sig OpGt   extends BinOp {}
one sig OpLte  extends BinOp {}
one sig OpGte  extends BinOp {}
one sig OpAnd  extends BinOp {}
one sig OpOr   extends BinOp {}

sig ExprBinOp extends ExprNode {
  binOp:    one BinOp,
  binLeft:  one ExprNode,
  binRight: one ExprNode
}

sig ExprNot extends ExprNode {
  notInner: one ExprNode
}

sig ExprIf extends ExprNode {
  ifCond:  one ExprNode,
  ifThen:  one ExprNode,
  ifElse:  one ExprNode
}

-- No ExprBinOp may reference itself via its left or right child (structural acyclicity).
fact NoCyclicBinOp {
  all e: ExprBinOp | e != e.binLeft and e != e.binRight
}

fact NoCyclicExprIf {
  all e: ExprIf | e != e.ifCond and e != e.ifThen and e != e.ifElse
}

fact NoCyclicExprNot {
  all e: ExprNot | e != e.notInner
}

fact NoCyclicTableRef {
  all e: ExprTableRef | e != e.tableKey
}

-------------------------------------------------------------------------------
-- Entity and CompoundState (Hierarchy)
-------------------------------------------------------------------------------

sig Entity {
  machines: set StateMachine
}

sig CompoundState {
  compoundParent:  one State,
  subMachines:     set StateMachine,
  suspendPolicy:   one SuspendPolicy,
  promotions:      set PortPromotion
}

-- Port Promotion (§4.4): a sub-SM's Output Port exposed at the parent SM's scope.
-- Enables parent-level Transitions to guard on sub-SM events.
sig PortPromotion {
  promotedPort:   one Port,
  promotionOwner: one CompoundState
}

-- The promoted port must belong to one of the CompoundState's sub-SMs.
fact PortPromotionOwnership {
  all pp: PortPromotion |
    pp.promotedPort in pp.promotionOwner.subMachines.ports
}

-- Promoted ports must be Output kind (only outputs can be promoted upward).
fact PortPromotionOutputOnly {
  all pp: PortPromotion |
    pp.promotedPort.portKind = PortKindOutput or
    pp.promotedPort.portKind = PortKindContinuousOutput
}

-------------------------------------------------------------------------------
-- Active Set
-------------------------------------------------------------------------------

sig ActiveSet {
  activeMachines: set StateMachine
}

-------------------------------------------------------------------------------
-- Structural facts
-------------------------------------------------------------------------------

-- Every SM's initial state must be one of its own states.
fact InitialStateOwned {
  all sm: StateMachine | sm.initialState in sm.states
}

-- The active state (when present) must belong to the SM's states.
fact ActiveStateOwned {
  all sm: StateMachine | sm.activeState in sm.states
}

-- Both endpoints of every Transition must belong to the same SM.
fact TransitionStatesOwned {
  all sm: StateMachine | all t: sm.transitions |
    t.source in sm.states and t.target in sm.states
}

-- Within a single SM, no two Transitions from the same source State
-- may share the same Priority. Ties are a design error.
fact UniqueTransitionPriority {
  all sm: StateMachine | all t1: sm.transitions | all t2: sm.transitions |
    t1 != t2 and t1.source = t2.source implies t1.priority != t2.priority
}

-- Connections are directional: source must be an output kind,
-- target must be an input kind.
fact ConnectionDirectionality {
  all c: Connection |
    (c.connSource.portKind = PortKindOutput or
     c.connSource.portKind = PortKindContinuousOutput) and
    (c.connTarget.portKind = PortKindInput or
     c.connTarget.portKind = PortKindContinuousInput)
}

-- Signal types must match across a Connection.
fact ConnectionSignalTypeCompat {
  all c: Connection | c.connSource.signalType = c.connTarget.signalType
}

-- An Entity must have at least one SM.
fact EntityHasMachines {
  all e: Entity | some e.machines
}

-- Active Set only contains SMs that belong to some Entity.
fact ActiveSetSubsetOfEntities {
  all aset: ActiveSet | all sm: aset.activeMachines |
    some e: Entity | sm in e.machines
}

-- Each CompoundState's sub-SMs must not own the parent State directly.
-- (Prevents trivial cycles in compound state hierarchy.)
fact SubMachineDoesNotOwnParentState {
  all cs: CompoundState | all sm: cs.subMachines |
    no s: sm.states | s = cs.compoundParent
}

-------------------------------------------------------------------------------
-- Predicates: core operations
-------------------------------------------------------------------------------

pred fireTransition[sm: one StateMachine, t: one Transition] {
  t in sm.transitions
  t.source = sm.activeState
}

pred deliverSignal[c: one Connection, s: one Signal] {
  s.signalType = c.connSource.signalType
  c.connSource.portKind = PortKindOutput
  c.connTarget.portKind = PortKindInput
}

pred spawnEntity[e: one Entity] {
  some e.machines
}

pred activateStateMachine[aset: one ActiveSet, sm: one StateMachine] {
  sm in aset.activeMachines
}

pred deactivateStateMachine[aset: one ActiveSet, sm: one StateMachine] {
  no x: aset.activeMachines | x = sm
}

-------------------------------------------------------------------------------
-- Safety assertions
-------------------------------------------------------------------------------

assert InitialStateAlwaysValid {
  all sm: StateMachine | sm.initialState in sm.states
}

assert ActiveStateAlwaysValid {
  all sm: StateMachine | sm.activeState in sm.states
}

assert TransitionEndpointsValid {
  all sm: StateMachine | all t: sm.transitions |
    t.source in sm.states and t.target in sm.states
}

assert ConnectionsAreDirectional {
  all c: Connection |
    (c.connSource.portKind = PortKindOutput or
     c.connSource.portKind = PortKindContinuousOutput)
}

assert ActiveSetBounded {
  all aset: ActiveSet | all sm: aset.activeMachines |
    some e: Entity | sm in e.machines
}

check InitialStateAlwaysValid   for 6
check ActiveStateAlwaysValid    for 6
check TransitionEndpointsValid  for 6
check ConnectionsAreDirectional for 6
check ActiveSetBounded          for 6
