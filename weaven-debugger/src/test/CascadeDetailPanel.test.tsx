import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import CascadeDetailPanel from "../components/CascadeDetailPanel";
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
    diffs: [],
  });
});

describe("CascadeDetailPanel", () => {
  it("shows 'No cascade' when no CascadeStep events", () => {
    render(<CascadeDetailPanel />);
    expect(screen.getByText(/no cascade this tick/i)).toBeInTheDocument();
  });

  it("shows cascade depth and signal count", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "CascadeStep", tick: 1, phase: "Propagate", depth: 1, queueSize: 2 } as any,
        { kind: "SignalDelivered", tick: 1, phase: "Propagate", depth: 1, sourceSm: { inner: 1 }, targetSm: { inner: 2 }, targetPort: { inner: 0 }, triggeredTransition: { inner: 5 } } as any,
        { kind: "SignalDelivered", tick: 1, phase: "Propagate", depth: 1, sourceSm: { inner: 1 }, targetSm: { inner: 3 }, targetPort: { inner: 0 }, triggeredTransition: null } as any,
      ],
      stateChanges: [],
      diffs: [],
    });

    render(<CascadeDetailPanel />);
    expect(screen.getByTestId("cascade-detail-panel")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument(); // depth
    expect(screen.getByText("(2 signals)")).toBeInTheDocument();
  });

  it("shows signal delivery details", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "CascadeStep", tick: 1, phase: "Propagate", depth: 1, queueSize: 1 } as any,
        { kind: "SignalDelivered", tick: 1, phase: "Propagate", depth: 1, sourceSm: { inner: 1 }, targetSm: { inner: 2 }, targetPort: { inner: 3 }, triggeredTransition: { inner: 10 } } as any,
      ],
      stateChanges: [],
      diffs: [],
    });

    render(<CascadeDetailPanel />);
    const deliveries = screen.getAllByTestId("signal-delivery");
    expect(deliveries).toHaveLength(1);
    expect(screen.getByText("SM(1)")).toBeInTheDocument();
    expect(screen.getByText("SM(2):3")).toBeInTheDocument();
    expect(screen.getByText("T(10)")).toBeInTheDocument();
  });

  it("shows 'no transition' for unmatched signal delivery", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "CascadeStep", tick: 1, phase: "Propagate", depth: 1, queueSize: 1 } as any,
        { kind: "SignalDelivered", tick: 1, phase: "Propagate", depth: 1, sourceSm: { inner: 1 }, targetSm: { inner: 2 }, targetPort: { inner: 0 }, triggeredTransition: null } as any,
      ],
      stateChanges: [],
      diffs: [],
    });

    render(<CascadeDetailPanel />);
    expect(screen.getByText("no transition")).toBeInTheDocument();
  });
});
