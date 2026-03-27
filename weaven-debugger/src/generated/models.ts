export type Bool = boolean;
export type Int = number;

export interface SmId {}

export interface StateId {}

export interface TransitionId {}

export interface PortId {}

export interface ConnectionId {}

export interface InteractionRuleId {}

export interface Tick {}

export type Phase = "Input" | "Evaluate" | "Execute" | "Propagate" | "Lifecycle" | "Output";

export interface GuardEvaluated {
  readonly kind: "GuardEvaluated";
  readonly tick: Tick;
  readonly phase: Phase;
  readonly transition: TransitionId;
  readonly smId: SmId;
  readonly result: Bool;
  readonly contextSnapshot: [string, number][] | null;
  readonly evalTree: EvalTreeNode | null;
}

export interface IrMatched {
  readonly kind: "IrMatched";
  readonly tick: Tick;
  readonly phase: Phase;
  readonly ruleId: InteractionRuleId;
  readonly participants: Set<SmId>;
}

export interface TransitionFired {
  readonly kind: "TransitionFired";
  readonly tick: Tick;
  readonly phase: Phase;
  readonly transition: TransitionId;
  readonly smId: SmId;
  readonly fromState: StateId;
  readonly toState: StateId;
}

export interface SignalEmitted {
  readonly kind: "SignalEmitted";
  readonly tick: Tick;
  readonly phase: Phase;
  readonly smId: SmId;
  readonly port: PortId;
  readonly target: SmId | null;
}

export interface CascadeStep {
  readonly kind: "CascadeStep";
  readonly tick: Tick;
  readonly phase: Phase;
  readonly depth: Int;
  readonly queueSize: Int;
}

export interface PipelineFiltered {
  readonly kind: "PipelineFiltered";
  readonly tick: Tick;
  readonly phase: Phase;
  readonly connection: ConnectionId | null;
  readonly smId: SmId;
  readonly port: PortId;
}

export interface SignalDelivered {
  readonly kind: "SignalDelivered";
  readonly tick: Tick;
  readonly phase: Phase;
  readonly depth: Int;
  readonly sourceSm: SmId | null;
  readonly targetSm: SmId;
  readonly targetPort: PortId;
  readonly triggeredTransition: TransitionId | null;
}

export type TraceEvent = GuardEvaluated | IrMatched | TransitionFired | SignalEmitted | CascadeStep | PipelineFiltered | SignalDelivered;

export type EdgeKind = "EdgeStatic" | "EdgeSpatial" | "EdgeIR";

export interface GraphNode {
  readonly sm: SmId;
  readonly activeState: StateId | null;
}

export interface GraphEdge {
  readonly edgeSource: GraphNode;
  readonly edgeTarget: GraphNode;
  readonly kind: EdgeKind;
  readonly connectionId: ConnectionId | null;
}

export interface TopologyGraph {
  readonly nodes: Set<GraphNode>;
  readonly edges: Set<GraphEdge>;
}

export type ExprKind = "ExprLit" | "ExprCtxRef" | "ExprSigRef" | "ExprTableRef" | "ExprBinOp" | "ExprNotOp" | "ExprIfOp" | "ExprPortRecv";

export interface EvalTreeNode {
  readonly exprKind: string;
  readonly label: string;
  readonly value: number;
  readonly children: EvalTreeNode[];
}

export interface Label {}

export interface EvalValue {}

export interface ContextSnapshot {
  readonly fields: Set<ContextEntry>;
}

export interface ContextEntry {
  readonly fieldName: Label;
  readonly fieldValue: EvalValue;
}

export interface SignalSnapshot {
  readonly signalFields: Set<ContextEntry>;
}

export interface GuardInspectionResult {
  readonly transition: TransitionId;
  readonly fired: Bool;
  readonly contextAtEval: ContextSnapshot;
  readonly signalAtEval: SignalSnapshot | null;
  readonly exprTree: EvalTreeNode;
}

export interface TickCursor {
  readonly current: Int;
  readonly maxTick: Int;
}

export interface WorldSnapshot {}

export interface FilterConfig {
  readonly visibleSms: Set<SmId>;
  readonly visibleConnections: Set<ConnectionId>;
  readonly visiblePhases: Set<Phase>;
}

export interface DebugSession {
  readonly snapshots: WorldSnapshot[];
  readonly cursor: TickCursor;
  readonly selectedSm: SmId | null;
  readonly trace: TraceEvent[];
  readonly topology: TopologyGraph;
  readonly filterCfg: FilterConfig;
}

export interface TickResult {
  readonly traceEvents: TraceEvent[];
  readonly stateChanges: Set<StateChange>;
}

export interface StateChange {
  readonly smId: SmId;
  readonly fromState: StateId;
  readonly toState: StateId;
}

export interface WorldState {
  readonly smStates: Set<SmStateEntry>;
}

export interface SmStateEntry {
  readonly smId: SmId;
  readonly activeState: StateId;
}

export interface SmStateDiff {
  readonly smId: number;
  readonly prevState: number;
  readonly newState: number;
  readonly contextChanges: Record<string, number>;
}
