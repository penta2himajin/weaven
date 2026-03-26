-- weaven-debugger.als
-- Domain model for the Weaven Debugger (Tauri-based debug & visualization tool).
-- Defines TraceEvent variants, DebugSession, TopologyGraph, and Inspector types.
-- Processed by oxidtr to generate Rust and TypeScript type definitions.
--
-- References: weaven-debugger-design.md §4

-------------------------------------------------------------------------------
-- Lightweight ID sigs (mirrors weaven-core runtime IDs)
-------------------------------------------------------------------------------

sig SmId {}
sig StateId {}
sig TransitionId {}
sig PortId {}
sig ConnectionId {}
sig InteractionRuleId {}

-------------------------------------------------------------------------------
-- Tick / Phase enums
-------------------------------------------------------------------------------

sig Tick {}

abstract sig Phase {}
one sig PhaseInput    extends Phase {}
one sig PhaseEvaluate extends Phase {}
one sig PhaseExecute  extends Phase {}
one sig PhasePropagate extends Phase {}
one sig PhaseLifecycle extends Phase {}
one sig PhaseOutput   extends Phase {}

-------------------------------------------------------------------------------
-- TraceEvent (§3.3 collection points)
-- abstract sig with shared fields: tick + phase
-- oxidtr generates Rust enum / TS discriminated union
-------------------------------------------------------------------------------

abstract sig TraceEvent {
  tick:  one Tick,
  phase: one Phase
}

-- Phase 2/4: Guard evaluated on a Transition
sig GuardEvaluated extends TraceEvent {
  transition: one TransitionId,
  smId:       one SmId,
  result:     one Bool
}

-- Phase 2: InteractionRule matched
sig IrMatched extends TraceEvent {
  ruleId:       one InteractionRuleId,
  participants: set SmId
}

-- Phase 3/4: Transition fired
sig TransitionFired extends TraceEvent {
  transition: one TransitionId,
  smId:       one SmId,
  fromState:  one StateId,
  toState:    one StateId
}

-- Phase 3/4: Signal emitted from an Output Port
sig SignalEmitted extends TraceEvent {
  smId:   one SmId,
  port:   one PortId,
  target: lone SmId
}

-- Phase 4: Each cascade iteration step
sig CascadeStep extends TraceEvent {
  depth:     one Int,
  queueSize: one Int
}

-- Phase 4: Signal blocked by Pipeline Filter
sig PipelineFiltered extends TraceEvent {
  connection: lone ConnectionId,
  smId:       one SmId,
  port:       one PortId
}

-- Ordering: phase must be valid for event type
fact GuardEvalPhase {
  all e: GuardEvaluated | e.phase in PhaseEvaluate + PhasePropagate
}

fact IrMatchPhase {
  all e: IrMatched | e.phase = PhaseEvaluate
}

fact TransitionFiredPhase {
  all e: TransitionFired | e.phase in PhaseExecute + PhasePropagate
}

fact SignalEmittedPhase {
  all e: SignalEmitted | e.phase in PhaseExecute + PhasePropagate
}

fact CascadeStepPhase {
  all e: CascadeStep | e.phase = PhasePropagate
}

fact PipelineFilteredPhase {
  all e: PipelineFiltered | e.phase = PhasePropagate
}

-- Cascade depth is non-negative
fact CascadeDepthNonNeg {
  all e: CascadeStep | e.depth >= 0
}

fact CascadeQueueNonNeg {
  all e: CascadeStep | e.queueSize >= 0
}

-------------------------------------------------------------------------------
-- TopologyGraph (§4.2 B)
-------------------------------------------------------------------------------

abstract sig EdgeKind {}
one sig EdgeStatic  extends EdgeKind {}
one sig EdgeSpatial extends EdgeKind {}
one sig EdgeIR      extends EdgeKind {}

sig GraphNode {
  sm:          one SmId,
  activeState: lone StateId
}

sig GraphEdge {
  edgeSource:   one GraphNode,
  edgeTarget:   one GraphNode,
  kind:         one EdgeKind,
  connectionId: lone ConnectionId
}

sig TopologyGraph {
  nodes: set GraphNode,
  edges: set GraphEdge
}

-- No self-loops
fact NoSelfLoop {
  no e: GraphEdge | e.edgeSource = e.edgeTarget
}

-- All edges reference nodes within the same graph
fact EdgesReferenceGraphNodes {
  all g: TopologyGraph | all e: g.edges |
    e.edgeSource in g.nodes and e.edgeTarget in g.nodes
}

-- Each node has a unique SM
fact UniqueSmPerNode {
  all g: TopologyGraph | all disj n1, n2: g.nodes | n1.sm != n2.sm
}

-------------------------------------------------------------------------------
-- Inspector: Guard evaluation tree (§4.2 D)
-------------------------------------------------------------------------------

abstract sig ExprKind {}
one sig ExprLit      extends ExprKind {}
one sig ExprCtxRef   extends ExprKind {}
one sig ExprSigRef   extends ExprKind {}
one sig ExprTableRef extends ExprKind {}
one sig ExprBinOp    extends ExprKind {}
one sig ExprNotOp    extends ExprKind {}
one sig ExprIfOp     extends ExprKind {}
one sig ExprPortRecv extends ExprKind {}

sig EvalTreeNode {
  exprKind: one ExprKind,
  label:    one Label,
  value:    one EvalValue,
  children: seq EvalTreeNode
}

sig Label {}
sig EvalValue {}

-- No cyclic eval tree
fact NoCyclicEvalTree {
  no n: EvalTreeNode | n in n.^children
}

sig ContextSnapshot {
  fields: set ContextEntry
}

sig ContextEntry {
  fieldName:  one Label,
  fieldValue: one EvalValue
}

sig SignalSnapshot {
  signalFields: set ContextEntry
}

sig GuardInspectionResult {
  transition:    one TransitionId,
  fired:         one Bool,
  contextAtEval: one ContextSnapshot,
  signalAtEval:  lone SignalSnapshot,
  exprTree:      one EvalTreeNode
}

-------------------------------------------------------------------------------
-- DebugSession (§4.2 A)
-------------------------------------------------------------------------------

sig TickCursor {
  current: one Int,
  maxTick: one Int
}

-- Cursor invariants
fact CursorRange {
  all c: TickCursor | c.current >= 0 and c.current <= c.maxTick
}

fact MaxTickNonNeg {
  all c: TickCursor | c.maxTick >= 0
}

sig WorldSnapshot {}

sig FilterConfig {
  visibleSms:         set SmId,
  visibleConnections: set ConnectionId,
  visiblePhases:      set Phase
}

sig DebugSession {
  snapshots:   seq WorldSnapshot,
  cursor:      one TickCursor,
  selectedSm:  lone SmId,
  trace:       seq TraceEvent,
  topology:    one TopologyGraph,
  filterCfg:   one FilterConfig
}

-- At least one snapshot (initial state)
fact SnapshotNonEmpty {
  all d: DebugSession | #d.snapshots > 0
}

-------------------------------------------------------------------------------
-- Tauri command result wrappers
-------------------------------------------------------------------------------

sig TickResult {
  traceEvents:  seq TraceEvent,
  stateChanges: set StateChange
}

sig StateChange {
  smId:      one SmId,
  fromState: one StateId,
  toState:   one StateId
}

sig WorldState {
  smStates: set SmStateEntry
}

sig SmStateEntry {
  smId:        one SmId,
  activeState: one StateId
}
