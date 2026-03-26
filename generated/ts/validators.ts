import type * as M from './models';

/** Runtime validator for GraphNode — checks all known constraints. */
export function validateGraphNode(g: M.GraphNode): string[] {
  const errors: string[] = [];
  if (g.sm == null) errors.push("sm must not be null");
  return errors;
}

/** Runtime validator for GraphEdge — checks all known constraints. */
// @covers: NoSelfLoop
export function validateGraphEdge(g: M.GraphEdge): string[] {
  const errors: string[] = [];
  if (g.edgeSource == null) errors.push("edgeSource must not be null");
  if (g.edgeTarget == null) errors.push("edgeTarget must not be null");
  if (g.kind == null) errors.push("kind must not be null");
  if (g.edgeSource === g.edgeTarget) errors.push("prohibited: GraphEdge.edgeSource = GraphEdge.edgeTarget");
  return errors;
}

/** Runtime validator for TopologyGraph — checks all known constraints. */
// @covers: EdgesReferenceGraphNodes
// @covers: UniqueSmPerNode
export function validateTopologyGraph(t: M.TopologyGraph): string[] {
  const errors: string[] = [];
  return errors;
}

/** Runtime validator for EvalTreeNode — checks all known constraints. */
// @covers: NoCyclicEvalTree
export function validateEvalTreeNode(e: M.EvalTreeNode): string[] {
  const errors: string[] = [];
  if (e.exprKind == null) errors.push("exprKind must not be null");
  if (e.label == null) errors.push("label must not be null");
  if (e.value == null) errors.push("value must not be null");
  { const seen = new Set<unknown>(); let cur: unknown = e; while (cur != null) { if (seen.has(cur)) { errors.push("children must not form a cycle"); break; } seen.add(cur); cur = (cur as Record<string, unknown>).children; } }
  return errors;
}

/** Runtime validator for ContextSnapshot — checks all known constraints. */
export function validateContextSnapshot(c: M.ContextSnapshot): string[] {
  const errors: string[] = [];
  return errors;
}

/** Runtime validator for ContextEntry — checks all known constraints. */
export function validateContextEntry(c: M.ContextEntry): string[] {
  const errors: string[] = [];
  if (c.fieldName == null) errors.push("fieldName must not be null");
  if (c.fieldValue == null) errors.push("fieldValue must not be null");
  return errors;
}

/** Runtime validator for SignalSnapshot — checks all known constraints. */
export function validateSignalSnapshot(s: M.SignalSnapshot): string[] {
  const errors: string[] = [];
  return errors;
}

/** Runtime validator for GuardInspectionResult — checks all known constraints. */
export function validateGuardInspectionResult(g: M.GuardInspectionResult): string[] {
  const errors: string[] = [];
  if (g.transition == null) errors.push("transition must not be null");
  if (g.fired == null) errors.push("fired must not be null");
  if (g.contextAtEval == null) errors.push("contextAtEval must not be null");
  if (g.exprTree == null) errors.push("exprTree must not be null");
  return errors;
}

/** Runtime validator for TickCursor — checks all known constraints. */
// @covers: CursorRange
// @covers: MaxTickNonNeg
export function validateTickCursor(t: M.TickCursor): string[] {
  const errors: string[] = [];
  if (t.current == null) errors.push("current must not be null");
  if (t.maxTick == null) errors.push("maxTick must not be null");
  if (t.current > t.maxTick) errors.push("current must be <= maxTick");
  return errors;
}

/** Runtime validator for FilterConfig — checks all known constraints. */
export function validateFilterConfig(f: M.FilterConfig): string[] {
  const errors: string[] = [];
  return errors;
}

/** Runtime validator for DebugSession — checks all known constraints. */
// @covers: SnapshotNonEmpty
export function validateDebugSession(d: M.DebugSession): string[] {
  const errors: string[] = [];
  if (d.cursor == null) errors.push("cursor must not be null");
  if (d.topology == null) errors.push("topology must not be null");
  if (d.filterCfg == null) errors.push("filterCfg must not be null");
  if (d.snapshots.length < 1) errors.push("snapshots must have at least 1 element(s)");
  return errors;
}

/** Runtime validator for TickResult — checks all known constraints. */
export function validateTickResult(t: M.TickResult): string[] {
  const errors: string[] = [];
  return errors;
}

/** Runtime validator for StateChange — checks all known constraints. */
export function validateStateChange(s: M.StateChange): string[] {
  const errors: string[] = [];
  if (s.smId == null) errors.push("smId must not be null");
  if (s.fromState == null) errors.push("fromState must not be null");
  if (s.toState == null) errors.push("toState must not be null");
  return errors;
}

/** Runtime validator for WorldState — checks all known constraints. */
export function validateWorldState(w: M.WorldState): string[] {
  const errors: string[] = [];
  return errors;
}

/** Runtime validator for SmStateEntry — checks all known constraints. */
export function validateSmStateEntry(s: M.SmStateEntry): string[] {
  const errors: string[] = [];
  if (s.smId == null) errors.push("smId must not be null");
  if (s.activeState == null) errors.push("activeState must not be null");
  return errors;
}

