import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import InspectorPanel from "../components/InspectorPanel";
import { useDebugStore } from "../stores/debugStore";
import { CommandsProvider } from "../components/CommandsContext";
import { createCommands } from "../commands";

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

describe("InspectorPanel", () => {
  it("shows prompt when no SM selected", () => {
    render(<InspectorPanel />);
    expect(screen.getByText(/select an sm/i)).toBeInTheDocument();
  });

  it("shows SM id when selected", () => {
    useDebugStore.getState().selectSm({ inner: 3 });
    render(<InspectorPanel />);
    expect(screen.getByText(/SM\(3\)/)).toBeInTheDocument();
  });

  it("shows guard results for selected SM", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 5 }, smId: { inner: 1 }, result: true } as any,
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 6 }, smId: { inner: 1 }, result: false } as any,
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 7 }, smId: { inner: 2 }, result: true } as any,
      ],
      stateChanges: [],
    });
    useDebugStore.getState().selectSm({ inner: 1 });

    render(<InspectorPanel />);
    // Should show 2 guard evaluations for SM(1), not the one for SM(2).
    const passMarks = screen.getAllByText("✓");
    const failMarks = screen.getAllByText("✗");
    expect(passMarks).toHaveLength(1);
    expect(failMarks).toHaveLength(1);
  });

  it("shows transition fired for selected SM", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "TransitionFired", tick: { inner: 1 }, phase: "Execute", transition: { inner: 0 }, smId: { inner: 1 }, fromState: { inner: 0 }, toState: { inner: 1 } } as any,
      ],
      stateChanges: [],
    });
    useDebugStore.getState().selectSm({ inner: 1 });

    render(<InspectorPanel />);
    expect(screen.getByText(/S\(0\) → S\(1\)/)).toBeInTheDocument();
  });

  it("shows 'None this tick' when no guards for selected SM", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "GuardEvaluated", tick: { inner: 1 }, phase: "Evaluate", transition: { inner: 0 }, smId: { inner: 2 }, result: true } as any,
      ],
      stateChanges: [],
    });
    useDebugStore.getState().selectSm({ inner: 99 });

    render(<InspectorPanel />);
    const nones = screen.getAllByText(/none this tick/i);
    expect(nones.length).toBeGreaterThanOrEqual(1);
  });

  it("renders inject signal form when SM is selected and commands available", () => {
    useDebugStore.getState().selectSm({ inner: 1 });

    render(
      <CommandsProvider commands={createCommands(vi.fn())}>
        <InspectorPanel />
      </CommandsProvider>,
    );

    expect(screen.getByText(/inject signal/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /inject/i })).toBeInTheDocument();
  });
});
