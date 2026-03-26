import { describe, it, expect, beforeEach } from "vitest";
import { useDebugStore } from "../stores/debugStore";

// Reset store between tests.
beforeEach(() => {
  useDebugStore.setState({
    loaded: false,
    currentTick: 0,
    maxTick: 0,
    topology: null,
    traceEvents: [],
    selectedSmId: null,
    cascadeIndex: 0,
    selectedTraceIndex: null,
    filterConfig: { hiddenSmIds: new Set(), hiddenPhases: new Set() },
  });
});

// --- Mock trace events ---
const guardEval = (smId: number, result: boolean) => ({
  kind: "GuardEvaluated" as const,
  tick: { inner: 1 },
  phase: "Evaluate",
  transition: { inner: 0 },
  smId: { inner: smId },
  result,
});

const transitionFired = (smId: number, from: number, to: number) => ({
  kind: "TransitionFired" as const,
  tick: { inner: 1 },
  phase: "Execute",
  transition: { inner: 0 },
  smId: { inner: smId },
  fromState: { inner: from },
  toState: { inner: to },
});

const cascadeStep = (depth: number, queueSize: number) => ({
  kind: "CascadeStep" as const,
  tick: { inner: 1 },
  phase: "Propagate",
  depth,
  queueSize,
});

describe("debugStore — applyTickResult", () => {
  it("updates currentTick and maxTick", () => {
    const { applyTickResult } = useDebugStore.getState();
    applyTickResult({
      tick: 5,
      traceEvents: [],
      stateChanges: [],
    });
    const state = useDebugStore.getState();
    expect(state.currentTick).toBe(5);
    expect(state.maxTick).toBe(5);
    expect(state.loaded).toBe(true);
  });

  it("stores trace events", () => {
    const events = [guardEval(1, true), transitionFired(1, 0, 1)];
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: events,
      stateChanges: [],
    });
    expect(useDebugStore.getState().traceEvents).toHaveLength(2);
  });

  it("maxTick only increases", () => {
    const apply = useDebugStore.getState().applyTickResult;
    apply({ tick: 10, traceEvents: [], stateChanges: [] });
    apply({ tick: 3, traceEvents: [], stateChanges: [] }); // seek back
    expect(useDebugStore.getState().maxTick).toBe(10);
  });
});

describe("debugStore — applySeeked", () => {
  it("updates currentTick without changing maxTick", () => {
    const store = useDebugStore.getState();
    store.applyTickResult({ tick: 10, traceEvents: [], stateChanges: [] });
    store.applySeeked({ tick: 3, smStates: [] });
    const state = useDebugStore.getState();
    expect(state.currentTick).toBe(3);
    expect(state.maxTick).toBe(10); // unchanged
  });

  it("clears trace events on seek", () => {
    const store = useDebugStore.getState();
    store.applyTickResult({
      tick: 1,
      traceEvents: [guardEval(1, true)],
      stateChanges: [],
    });
    store.applySeeked({ tick: 0, smStates: [] });
    expect(useDebugStore.getState().traceEvents).toHaveLength(0);
  });
});

describe("debugStore — filteredTraceEvents", () => {
  it("returns all events when no SM selected", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [guardEval(1, true), guardEval(2, false)],
      stateChanges: [],
    });
    const filtered = useDebugStore.getState().filteredTraceEvents();
    expect(filtered).toHaveLength(2);
  });

  it("filters by selected SM", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        guardEval(1, true),
        guardEval(2, false),
        transitionFired(1, 0, 1),
      ],
      stateChanges: [],
    });
    useDebugStore.getState().selectSm({ inner: 1 });
    const filtered = useDebugStore.getState().filteredTraceEvents();
    expect(filtered).toHaveLength(2); // SM(1) guard + transition
    expect(filtered.every((e: any) => e.smId?.inner === 1)).toBe(true);
  });

  it("includes non-SM events (CascadeStep) when SM selected", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        guardEval(1, true),
        cascadeStep(1, 3),
        guardEval(2, false),
      ],
      stateChanges: [],
    });
    useDebugStore.getState().selectSm({ inner: 1 });
    const filtered = useDebugStore.getState().filteredTraceEvents();
    // SM(1) guard + CascadeStep (no smId, always included)
    expect(filtered).toHaveLength(2);
  });
});

describe("debugStore — cascade navigation", () => {
  it("cascadeSteps extracts CascadeStep events", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        guardEval(1, true),
        cascadeStep(1, 5),
        cascadeStep(2, 3),
        transitionFired(1, 0, 1),
      ],
      stateChanges: [],
    });
    const steps = useDebugStore.getState().cascadeSteps();
    expect(steps).toHaveLength(2);
    expect(steps[0].kind).toBe("CascadeStep");
  });

  it("nextCascadeStep advances index", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [cascadeStep(1, 5), cascadeStep(2, 3), cascadeStep(3, 1)],
      stateChanges: [],
    });
    expect(useDebugStore.getState().cascadeIndex).toBe(0);
    useDebugStore.getState().nextCascadeStep();
    expect(useDebugStore.getState().cascadeIndex).toBe(1);
    useDebugStore.getState().nextCascadeStep();
    expect(useDebugStore.getState().cascadeIndex).toBe(2);
    // Should clamp at max.
    useDebugStore.getState().nextCascadeStep();
    expect(useDebugStore.getState().cascadeIndex).toBe(2);
  });

  it("prevCascadeStep decrements index", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [cascadeStep(1, 5), cascadeStep(2, 3)],
      stateChanges: [],
    });
    useDebugStore.getState().nextCascadeStep();
    expect(useDebugStore.getState().cascadeIndex).toBe(1);
    useDebugStore.getState().prevCascadeStep();
    expect(useDebugStore.getState().cascadeIndex).toBe(0);
    // Should clamp at 0.
    useDebugStore.getState().prevCascadeStep();
    expect(useDebugStore.getState().cascadeIndex).toBe(0);
  });
});

// =========================================================================
// Signal flow highlight
// =========================================================================

const signalEmitted = (smId: number, port: number, target: number) => ({
  kind: "SignalEmitted" as const,
  tick: { inner: 1 },
  phase: "Execute",
  smId: { inner: smId },
  port: { inner: port },
  target: { inner: target },
});

const pipelineFiltered = (smId: number, connId: number) => ({
  kind: "PipelineFiltered" as const,
  tick: { inner: 1 },
  phase: "Propagate",
  connection: { inner: connId },
  smId: { inner: smId },
  port: { inner: 0 },
});

describe("debugStore — signal flow highlight", () => {
  it("selectTraceEvent stores the selected event index", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [guardEval(1, true), signalEmitted(1, 1, 2)],
      stateChanges: [],
    });
    useDebugStore.getState().selectTraceEvent(1);
    expect(useDebugStore.getState().selectedTraceIndex).toBe(1);
  });

  it("clearTraceSelection resets to null", () => {
    useDebugStore.getState().selectTraceEvent(1);
    useDebugStore.getState().clearTraceSelection();
    expect(useDebugStore.getState().selectedTraceIndex).toBeNull();
  });

  it("highlightedEdges returns source→target for SignalEmitted", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [guardEval(1, true), signalEmitted(1, 1, 2)],
      stateChanges: [],
    });
    useDebugStore.getState().selectTraceEvent(1); // SignalEmitted

    const edges = useDebugStore.getState().highlightedEdges();
    expect(edges).toHaveLength(1);
    expect(edges[0]).toEqual({ source: 1, target: 2, kind: "signal" });
  });

  it("highlightedEdges returns blocked edge for PipelineFiltered", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [pipelineFiltered(3, 5)],
      stateChanges: [],
    });
    useDebugStore.getState().selectTraceEvent(0);

    const edges = useDebugStore.getState().highlightedEdges();
    expect(edges).toHaveLength(1);
    expect(edges[0].kind).toBe("filtered");
  });

  it("highlightedEdges returns empty when no selection", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [signalEmitted(1, 1, 2)],
      stateChanges: [],
    });
    // No selection.
    const edges = useDebugStore.getState().highlightedEdges();
    expect(edges).toHaveLength(0);
  });

  it("highlightedEdges returns empty for non-signal events", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [guardEval(1, true)],
      stateChanges: [],
    });
    useDebugStore.getState().selectTraceEvent(0); // GuardEvaluated — no edge

    const edges = useDebugStore.getState().highlightedEdges();
    expect(edges).toHaveLength(0);
  });
});

// =========================================================================
// Edge style computation
// =========================================================================

import { computeEdgeStyles } from "../components/edgeStyles";

describe("computeEdgeStyles", () => {
  it("returns empty map when no highlights", () => {
    const styles = computeEdgeStyles([], []);
    expect(Object.keys(styles)).toHaveLength(0);
  });

  it("marks signal edge as highlighted", () => {
    const edges = [
      { id: "edge-0", source: "sm-1", target: "sm-2" },
    ];
    const highlights = [{ source: 1, target: 2, kind: "signal" as const }];
    const styles = computeEdgeStyles(edges, highlights);

    expect(styles["edge-0"]).toBeDefined();
    expect(styles["edge-0"].stroke).toBe("#22d3ee");
    expect(styles["edge-0"].strokeWidth).toBe(4);
  });

  it("marks filtered edge with red dashed style", () => {
    const edges = [
      { id: "edge-0", source: "sm-1", target: "sm-3" },
    ];
    // PipelineFiltered: target SM is 3, self-referencing edge
    const highlights = [{ source: 3, target: 3, kind: "filtered" as const }];
    const styles = computeEdgeStyles(edges, highlights);

    // Edge targeting sm-3 should be marked
    expect(styles["edge-0"]).toBeDefined();
    expect(styles["edge-0"].stroke).toBe("#ef4444");
  });

  it("does not mark unrelated edges", () => {
    const edges = [
      { id: "edge-0", source: "sm-1", target: "sm-2" },
      { id: "edge-1", source: "sm-3", target: "sm-4" },
    ];
    const highlights = [{ source: 1, target: 2, kind: "signal" as const }];
    const styles = computeEdgeStyles(edges, highlights);

    expect(styles["edge-0"]).toBeDefined();
    expect(styles["edge-1"]).toBeUndefined();
  });
});

// =========================================================================
// FilterConfig
// =========================================================================

describe("debugStore — filterConfig", () => {
  it("default filter shows everything", () => {
    const { filterConfig } = useDebugStore.getState();
    expect(filterConfig.hiddenSmIds).toEqual(new Set());
    expect(filterConfig.hiddenPhases).toEqual(new Set());
  });

  it("toggleSmVisibility hides and unhides an SM", () => {
    useDebugStore.getState().toggleSmVisibility(1);
    expect(useDebugStore.getState().filterConfig.hiddenSmIds.has(1)).toBe(true);

    useDebugStore.getState().toggleSmVisibility(1);
    expect(useDebugStore.getState().filterConfig.hiddenSmIds.has(1)).toBe(false);
  });

  it("togglePhaseVisibility hides and unhides a phase", () => {
    useDebugStore.getState().togglePhaseVisibility("Evaluate");
    expect(useDebugStore.getState().filterConfig.hiddenPhases.has("Evaluate")).toBe(true);

    useDebugStore.getState().togglePhaseVisibility("Evaluate");
    expect(useDebugStore.getState().filterConfig.hiddenPhases.has("Evaluate")).toBe(false);
  });

  it("filteredTraceEvents respects hidden phases", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        guardEval(1, true),   // phase: Evaluate
        transitionFired(1, 0, 1), // phase: Execute
        cascadeStep(1, 3),    // phase: Propagate
      ],
      stateChanges: [],
    });
    useDebugStore.getState().togglePhaseVisibility("Propagate");

    const filtered = useDebugStore.getState().filteredTraceEvents();
    // CascadeStep (Propagate) should be hidden.
    expect(filtered).toHaveLength(2);
    expect(filtered.every((e: any) => e.phase !== "Propagate")).toBe(true);
  });

  it("filteredTraceEvents respects hidden SMs", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        guardEval(1, true),
        guardEval(2, false),
        transitionFired(1, 0, 1),
      ],
      stateChanges: [],
    });
    useDebugStore.getState().toggleSmVisibility(2);

    const filtered = useDebugStore.getState().filteredTraceEvents();
    // SM(2) events should be hidden.
    expect(filtered).toHaveLength(2);
    expect(filtered.every((e: any) => !e.smId || e.smId.inner !== 2)).toBe(true);
  });

  it("resetFilter clears all filters", () => {
    useDebugStore.getState().toggleSmVisibility(1);
    useDebugStore.getState().togglePhaseVisibility("Execute");
    useDebugStore.getState().resetFilter();

    const { filterConfig } = useDebugStore.getState();
    expect(filterConfig.hiddenSmIds.size).toBe(0);
    expect(filterConfig.hiddenPhases.size).toBe(0);
  });
});
