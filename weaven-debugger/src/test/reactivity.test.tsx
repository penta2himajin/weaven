/**
 * Category 4: Zustand reactivity tests.
 *
 * Verify that components re-render when store state changes.
 * The key pattern: render → update store → assert component reflects new state.
 *
 * This catches the bug where subscribing to a derived function (like
 * filteredTraceEvents) doesn't trigger re-render when the underlying
 * data (traceEvents) changes.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, act } from "@testing-library/react";
import TracePanel from "../components/TracePanel";
import InspectorPanel from "../components/InspectorPanel";
import TimelinePanel from "../components/TimelinePanel";
import { CommandsProvider } from "../components/CommandsContext";
import { createCommands } from "../commands";
import { useDebugStore } from "../stores/debugStore";
import type { TraceEvent } from "../generated/models";

beforeEach(() => {
  useDebugStore.setState({
    loaded: true,
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

const mockEvent = (smId: number): TraceEvent => ({
  kind: "GuardEvaluated" as const,
  tick: { inner: 1 },
  phase: "Evaluate" as const,
  transition: { inner: 0 },
  smId: { inner: smId },
  result: true,
} as any);

const transitionEvent = (smId: number): TraceEvent => ({
  kind: "TransitionFired" as const,
  tick: { inner: 1 },
  phase: "Execute" as const,
  transition: { inner: 0 },
  smId: { inner: smId },
  fromState: { inner: 0 },
  toState: { inner: 1 },
} as any);

describe("reactivity — TracePanel", () => {
  it("updates when traceEvents change after initial render", () => {
    render(<TracePanel />);
    expect(screen.getByText(/no trace events/i)).toBeInTheDocument();

    // Update store AFTER render.
    act(() => {
      useDebugStore.getState().applyTickResult({
        tick: 1,
        traceEvents: [mockEvent(1), mockEvent(2)],
        stateChanges: [],
      });
    });

    expect(screen.getByText("2 events")).toBeInTheDocument();
  });

  it("updates when selectedSmId changes (filter narrows)", () => {
    act(() => {
      useDebugStore.getState().applyTickResult({
        tick: 1,
        traceEvents: [mockEvent(1), mockEvent(2), mockEvent(1)],
        stateChanges: [],
      });
    });

    render(<TracePanel />);
    expect(screen.getByText("3 events")).toBeInTheDocument();

    // Select SM(1) — should filter to 2 events.
    act(() => {
      useDebugStore.getState().selectSm({ inner: 1 });
    });

    expect(screen.getByText("2 events")).toBeInTheDocument();
  });
});

describe("reactivity — InspectorPanel", () => {
  it("updates guard display when traceEvents change", () => {
    // Select SM first.
    act(() => {
      useDebugStore.getState().selectSm({ inner: 1 });
    });

    render(<InspectorPanel />);
    const nones = screen.getAllByText(/none this tick/i);
    expect(nones.length).toBeGreaterThanOrEqual(1);

    // Tick updates trace events.
    act(() => {
      useDebugStore.getState().applyTickResult({
        tick: 1,
        traceEvents: [mockEvent(1), transitionEvent(1)],
        stateChanges: [],
      });
    });

    // Should now show guard result and transition.
    expect(screen.getByText("✓")).toBeInTheDocument();
    expect(screen.getByText(/S\(0\) → S\(1\)/)).toBeInTheDocument();
  });

  it("updates when selectedSm changes", () => {
    act(() => {
      useDebugStore.getState().applyTickResult({
        tick: 1,
        traceEvents: [mockEvent(1), mockEvent(2)],
        stateChanges: [],
      });
    });

    render(<InspectorPanel />);
    expect(screen.getByText(/select an sm/i)).toBeInTheDocument();

    act(() => {
      useDebugStore.getState().selectSm({ inner: 1 });
    });

    expect(screen.getByText(/SM\(1\)/)).toBeInTheDocument();
  });
});

describe("reactivity — TimelinePanel", () => {
  it("tick display updates when currentTick changes", () => {
    const invoke = vi.fn().mockResolvedValue({ tick: 5, trace_events: [], state_changes: [] });
    render(
      <CommandsProvider commands={createCommands(invoke)}>
        <TimelinePanel />
      </CommandsProvider>,
    );

    // Initial state: "Tick 0 / 0"
    expect(screen.getByTestId("tick-display").textContent).toContain("0");

    act(() => {
      useDebugStore.getState().applyTickResult({
        tick: 42,
        traceEvents: [],
        stateChanges: [],
      });
    });

    // After update: "Tick 42 / 42"
    expect(screen.getByTestId("tick-display").textContent).toContain("42");
  });
});

const signalEvent = (srcSmId: number, tgtSmId: number): TraceEvent => ({
  kind: "SignalEmitted" as const,
  tick: { inner: 1 },
  phase: "Execute" as const,
  smId: { inner: srcSmId },
  port: { inner: 0 },
  target: { inner: tgtSmId },
} as any);

describe("reactivity — TopologyCanvas highlight", () => {
  it("edge becomes animated when a SignalEmitted trace event is selected", async () => {
    const { default: TopologyCanvas } = await import("../components/TopologyCanvas");

    act(() => {
      useDebugStore.getState().setTopology({
        nodes: [
          { sm_id: { inner: 1 }, active_state: { inner: 0 }, label: "SM(1)" },
          { sm_id: { inner: 2 }, active_state: { inner: 0 }, label: "SM(2)" },
        ],
        edges: [
          { source: { inner: 1 }, target: { inner: 2 }, kind: "Static", connection_id: null, label: "" },
        ],
      });
      useDebugStore.getState().applyTickResult({
        tick: 1,
        traceEvents: [signalEvent(1, 2)],
        stateChanges: [],
      });
    });

    render(<TopologyCanvas />);

    // Before selection — no highlighted edges.
    expect(useDebugStore.getState().highlightedEdges()).toHaveLength(0);

    // Select the SignalEmitted event.
    act(() => {
      useDebugStore.getState().selectTraceEvent(0);
    });

    // highlightedEdges() should now return the 1→2 edge.
    const hl = useDebugStore.getState().highlightedEdges();
    expect(hl).toHaveLength(1);
    expect(hl[0]).toMatchObject({ source: 1, target: 2, kind: "signal" });
  });

  it("TopologyCanvas re-renders edges when selectedTraceIndex changes", async () => {
    const { default: TopologyCanvas } = await import("../components/TopologyCanvas");

    act(() => {
      useDebugStore.getState().setTopology({
        nodes: [
          { sm_id: { inner: 1 }, active_state: { inner: 0 }, label: "SM(1)" },
          { sm_id: { inner: 2 }, active_state: { inner: 0 }, label: "SM(2)" },
        ],
        edges: [
          { source: { inner: 1 }, target: { inner: 2 }, kind: "Static", connection_id: null, label: "" },
        ],
      });
      useDebugStore.getState().applyTickResult({
        tick: 1,
        traceEvents: [signalEvent(1, 2)],
        stateChanges: [],
      });
    });

    const { container } = render(<TopologyCanvas />);

    // No selection yet — component should render without crashing.
    expect(container.firstChild).not.toBeNull();

    // Selecting trace event should not crash either.
    act(() => {
      useDebugStore.getState().selectTraceEvent(0);
    });

    expect(container.firstChild).not.toBeNull();
  });
});

const guardFailEvent = (): TraceEvent => ({
  kind: "GuardEvaluated" as const,
  tick: { inner: 1 },
  phase: "Evaluate" as const,
  transition: { inner: 3 },
  smId: { inner: 1 },
  result: false,
  contextSnapshot: [["hp", 0], ["maxHp", 100]] as any,
} as any);

describe("reactivity — InspectorPanel context snapshot", () => {
  it("shows context snapshot fields when guard fails with snapshot", () => {
    act(() => {
      useDebugStore.getState().selectSm({ inner: 1 });
      useDebugStore.getState().applyTickResult({
        tick: 1,
        traceEvents: [guardFailEvent()],
        stateChanges: [],
      });
    });

    render(<InspectorPanel />);
    // Context fields from snapshot should be visible.
    expect(screen.getByText(/hp/)).toBeInTheDocument();
    expect(screen.getAllByText(/0/).length).toBeGreaterThanOrEqual(1);
  });
});
