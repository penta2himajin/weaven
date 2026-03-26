/**
 * Category 1: App wiring / integration tests.
 *
 * Verify that the app component tree is correctly wired:
 * - CommandsProvider must wrap the app
 * - WelcomeScreen shown when not loaded
 * - Main layout shown when loaded with topology
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "../App";
import { CommandsProvider, useCommands } from "../components/CommandsContext";
import { createCommands } from "../commands";
import { useDebugStore } from "../stores/debugStore";

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

function renderApp() {
  const mockInvoke = vi.fn().mockResolvedValue({});
  return render(
    <CommandsProvider commands={createCommands(mockInvoke)}>
      <App />
    </CommandsProvider>,
  );
}

describe("App wiring", () => {
  it("shows WelcomeScreen when not loaded", () => {
    renderApp();
    expect(screen.getByText(/weaven debugger/i)).toBeInTheDocument();
    expect(screen.getByText(/drop a weaven-schema/i)).toBeInTheDocument();
  });

  it("shows main layout when loaded with topology", () => {
    useDebugStore.setState({
      loaded: true,
      topology: {
        nodes: [{ sm_id: 1, active_state: 0, label: "SM(1)" }],
        edges: [],
      },
    });

    renderApp();
    // TimelinePanel should be present with Tick button.
    expect(screen.getByRole("button", { name: /^Tick$/i })).toBeInTheDocument();
    // TracePanel header should be present.
    expect(screen.getByText("Trace")).toBeInTheDocument();
    // InspectorPanel should show the "select an SM" prompt.
    expect(screen.getByText(/select an sm/i)).toBeInTheDocument();
  });

  it("useCommands throws without CommandsProvider", () => {
    const TestComp = () => {
      useCommands();
      return null;
    };
    // Suppress React error boundary console noise.
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => {
      render(<TestComp />);
    }).toThrow(/useCommands must be used within CommandsProvider/);
    spy.mockRestore();
  });

  it("shows FilterConfigPanel Reset button when loaded", () => {
    useDebugStore.setState({
      loaded: true,
      topology: {
        nodes: [{ sm_id: { inner: 1 }, active_state: { inner: 0 }, label: "SM(1)" }],
        edges: [],
      },
    });
    renderApp();
    expect(screen.getByRole("button", { name: /reset filters/i })).toBeInTheDocument();
  });

  it("shows FilterConfigPanel phase toggles when loaded", () => {
    useDebugStore.setState({
      loaded: true,
      topology: { nodes: [], edges: [] },
    });
    renderApp();
    expect(screen.getByText("Eval")).toBeInTheDocument();
    expect(screen.getByText("Exec")).toBeInTheDocument();
    expect(screen.getByText("Prop")).toBeInTheDocument();
  });

  it("transitions from WelcomeScreen to main layout on loadSchema", async () => {
    const mockInvoke = vi.fn().mockResolvedValue({
      nodes: [{ sm_id: 1, active_state: 0, label: "SM(1)" }],
      edges: [],
    });

    const { rerender } = render(
      <CommandsProvider commands={createCommands(mockInvoke)}>
        <App />
      </CommandsProvider>,
    );

    // Initially shows welcome.
    expect(screen.getByText(/weaven debugger/i)).toBeInTheDocument();

    // Simulate loadSchema completing — setTopology sets loaded: true.
    useDebugStore.getState().setTopology({
      nodes: [{ sm_id: { inner: 1 }, active_state: { inner: 0 }, label: "SM(1)" }],
      edges: [],
    });

    rerender(
      <CommandsProvider commands={createCommands(mockInvoke)}>
        <App />
      </CommandsProvider>,
    );

    // Should now show main layout.
    expect(screen.getByRole("button", { name: /^Tick$/i })).toBeInTheDocument();
  });
});
