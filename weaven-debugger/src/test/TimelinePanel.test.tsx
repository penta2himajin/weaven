import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import TimelinePanel from "../components/TimelinePanel";
import { useDebugStore } from "../stores/debugStore";
import { CommandsProvider } from "../components/CommandsContext";
import { createCommands, type TauriInvoke } from "../commands";

beforeEach(() => {
  useDebugStore.setState({
    loaded: true,
    currentTick: 5,
    maxTick: 20,
    topology: null,
    traceEvents: [],
    selectedSmId: null,
    cascadeIndex: 0,
  });
});

function renderWithCommands(mockInvoke: TauriInvoke) {
  const cmds = createCommands(mockInvoke);
  return render(
    <CommandsProvider commands={cmds}>
      <TimelinePanel />
    </CommandsProvider>,
  );
}

describe("TimelinePanel", () => {
  it("displays current tick and max tick", () => {
    const invoke = vi.fn().mockResolvedValue({ tick: 5, trace_events: [], state_changes: [] });
    renderWithCommands(invoke);

    expect(screen.getByText("5")).toBeInTheDocument();
    expect(screen.getByText(/\/ 20/)).toBeInTheDocument();
  });

  it("Tick button calls tick command", async () => {
    const invoke = vi.fn().mockResolvedValue({ tick: 6, trace_events: [], state_changes: [] });
    renderWithCommands(invoke);

    fireEvent.click(screen.getByRole("button", { name: /^Tick$/i }));
    // Tick button should call invoke("tick").
    expect(invoke).toHaveBeenCalledWith("tick");
  });

  it("×10 button calls tickN(10)", async () => {
    const invoke = vi.fn().mockResolvedValue({ tick: 15, trace_events: [], state_changes: [] });
    renderWithCommands(invoke);

    fireEvent.click(screen.getByRole("button", { name: /×10/i }));
    expect(invoke).toHaveBeenCalledWith("tick_n", { n: 10 });
  });

  it("cascade ▶ button calls nextCascadeStep", () => {
    useDebugStore.getState().applyTickResult({
      tick: 1,
      traceEvents: [
        { kind: "CascadeStep", tick: { inner: 1 }, phase: "Propagate", depth: 1, queueSize: 3 } as any,
        { kind: "CascadeStep", tick: { inner: 1 }, phase: "Propagate", depth: 2, queueSize: 1 } as any,
      ],
      stateChanges: [],
    });

    const invoke = vi.fn();
    renderWithCommands(invoke);

    const cascadeNext = screen.getByRole("button", { name: /next cascade/i });
    fireEvent.click(cascadeNext);
    expect(useDebugStore.getState().cascadeIndex).toBe(1);
  });
});
