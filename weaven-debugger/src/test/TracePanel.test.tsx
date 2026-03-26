import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import TracePanel from "../components/TracePanel";
import { useDebugStore } from "../stores/debugStore";

beforeEach(() => {
  useDebugStore.setState({
    loaded: true,
    currentTick: 1,
    maxTick: 1,
    topology: null,
    traceEvents: [],
    selectedSmId: null,
    cascadeIndex: 0,
  });
});

describe("TracePanel", () => {
  it("shows 'No trace events' when empty", () => {
    render(<TracePanel />);
    expect(screen.getByText(/no trace events/i)).toBeInTheDocument();
  });

  it("renders event count badge", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 0 }, smId: { inner: 1 }, result: true } as any,
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 1 }, smId: { inner: 2 }, result: false } as any,
      ],
      stateChanges: [],
    });
    render(<TracePanel />);
    expect(screen.getByText("2 events")).toBeInTheDocument();
  });

  it("filters events when SM is selected", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 0 }, smId: { inner: 1 }, result: true } as any,
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 1 }, smId: { inner: 2 }, result: false } as any,
        { kind: "TransitionFired", tick: { inner: 1 }, phase: "Execute", transition: { inner: 0 }, smId: { inner: 1 }, fromState: { inner: 0 }, toState: { inner: 1 } } as any,
      ],
      stateChanges: [],
    });
    useDebugStore.getState().selectSm({ inner: 1 });

    render(<TracePanel />);
    // Only SM(1) events: 2 out of 3.
    expect(screen.getByText("2 events")).toBeInTheDocument();
  });
});

  it("clicking a trace row selects it in the store", async () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 0 }, smId: { inner: 1 }, result: true, context_snapshot: null } as any,
        { kind: "SignalEmitted", tick: { inner: 1 }, phase: "Execute", smId: { inner: 1 }, port: { inner: 1 }, target: { inner: 2 } } as any,
      ],
      stateChanges: [],
    });
    const { container } = render(<TracePanel />);

    // Click the second row (SignalEmitted).
    const rows = container.querySelectorAll("[data-trace-index]");
    expect(rows.length).toBe(2);

    await fireEvent.click(rows[1]);
    expect(useDebugStore.getState().selectedTraceIndex).toBe(1);
  });
