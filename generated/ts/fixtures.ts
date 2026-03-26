import type * as M from './models';

/** Factory: default value for enum Phase */
export function defaultPhase(): M.Phase {
  return "PhaseInput";
}

/** Factory: default value for enum EdgeKind */
export function defaultEdgeKind(): M.EdgeKind {
  return "EdgeStatic";
}

/** Factory: default value for enum ExprKind */
export function defaultExprKind(): M.ExprKind {
  return "ExprLit";
}

/** Factory: create a default valid GraphNode */
export function defaultGraphNode(): M.GraphNode {
  return {
    sm: defaultSmId(),
    activeState: null,
  };
}

/** Factory: create a default valid GraphEdge */
export function defaultGraphEdge(): M.GraphEdge {
  return {
    edgeSource: defaultGraphNode(),
    edgeTarget: defaultGraphNode(),
    kind: defaultEdgeKind(),
    connectionId: null,
  };
}

/** Factory: create a default valid TopologyGraph */
export function defaultTopologyGraph(): M.TopologyGraph {
  return {
    nodes: new Set([defaultGraphNode()]),
    edges: new Set([defaultGraphEdge()]),
  };
}

/** Factory: create a default valid EvalTreeNode */
export function defaultEvalTreeNode(): M.EvalTreeNode {
  return {
    exprKind: defaultExprKind(),
    label: defaultLabel(),
    value: defaultEvalValue(),
    children: [],
  };
}

/** Factory: create a default valid ContextSnapshot */
export function defaultContextSnapshot(): M.ContextSnapshot {
  return {
    fields: new Set([defaultContextEntry()]),
  };
}

/** Factory: create a default valid ContextEntry */
export function defaultContextEntry(): M.ContextEntry {
  return {
    fieldName: defaultLabel(),
    fieldValue: defaultEvalValue(),
  };
}

/** Factory: create a default valid SignalSnapshot */
export function defaultSignalSnapshot(): M.SignalSnapshot {
  return {
    signalFields: new Set([defaultContextEntry()]),
  };
}

/** Factory: create a default valid GuardInspectionResult */
export function defaultGuardInspectionResult(): M.GuardInspectionResult {
  return {
    transition: defaultTransitionId(),
    fired: defaultBool(),
    contextAtEval: defaultContextSnapshot(),
    signalAtEval: null,
    exprTree: defaultEvalTreeNode(),
  };
}

/** Factory: create a default valid TickCursor */
export function defaultTickCursor(): M.TickCursor {
  return {
    current: defaultInt(),
    maxTick: defaultInt(),
  };
}

/** Factory: create a default valid FilterConfig */
export function defaultFilterConfig(): M.FilterConfig {
  return {
    visibleSms: new Set(),
    visibleConnections: new Set(),
    visiblePhases: new Set(),
  };
}

/** Factory: create a default valid DebugSession */
export function defaultDebugSession(): M.DebugSession {
  return {
    snapshots: [],
    cursor: defaultTickCursor(),
    selectedSm: null,
    trace: [],
    topology: defaultTopologyGraph(),
    filterCfg: defaultFilterConfig(),
  };
}

/** Factory: create DebugSession at cardinality boundary */
export function boundaryDebugSession(): M.DebugSession {
  return {
    snapshots: [defaultWorldSnapshot()],
    cursor: defaultTickCursor(),
    selectedSm: null,
    trace: [],
    topology: defaultTopologyGraph(),
    filterCfg: defaultFilterConfig(),
  };
}

/** Factory: create DebugSession that violates cardinality constraint */
export function invalidDebugSession(): M.DebugSession {
  return {
    snapshots: [],
    cursor: defaultTickCursor(),
    selectedSm: null,
    trace: [],
    topology: defaultTopologyGraph(),
    filterCfg: defaultFilterConfig(),
  };
}

/** Factory: create a default valid TickResult */
export function defaultTickResult(): M.TickResult {
  return {
    traceEvents: [],
    stateChanges: new Set([defaultStateChange()]),
  };
}

/** Factory: create a default valid StateChange */
export function defaultStateChange(): M.StateChange {
  return {
    smId: defaultSmId(),
    fromState: defaultStateId(),
    toState: defaultStateId(),
  };
}

/** Factory: create a default valid WorldState */
export function defaultWorldState(): M.WorldState {
  return {
    smStates: new Set([defaultSmStateEntry()]),
  };
}

/** Factory: create a default valid SmStateEntry */
export function defaultSmStateEntry(): M.SmStateEntry {
  return {
    smId: defaultSmId(),
    activeState: defaultStateId(),
  };
}

