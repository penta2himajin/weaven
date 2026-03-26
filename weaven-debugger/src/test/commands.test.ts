import { describe, it, expect, vi, beforeEach } from "vitest";
import { createCommands, type TauriInvoke } from "../commands";
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
  });
});

describe("commands — tick", () => {
  it("calls invoke('tick') and applies result to store", async () => {
    const mockInvoke: TauriInvoke = vi.fn().mockResolvedValue({
      tick: 1,
      trace_events: [
        {
          kind: "GuardEvaluated",
          tick: 1,
          phase: "Evaluate",
          transition: 0,
          sm_id: 1,
          result: true,
        },
      ],
      state_changes: [],
    });

    const cmds = createCommands(mockInvoke);
    await cmds.tick();

    expect(mockInvoke).toHaveBeenCalledWith("tick");
    expect(useDebugStore.getState().currentTick).toBe(1);
    expect(useDebugStore.getState().traceEvents).toHaveLength(1);
  });
});

describe("commands — tickN", () => {
  it("calls invoke('tick_n') with count", async () => {
    const mockInvoke: TauriInvoke = vi.fn().mockResolvedValue({
      tick: 10,
      trace_events: [],
      state_changes: [],
    });

    const cmds = createCommands(mockInvoke);
    await cmds.tickN(10);

    expect(mockInvoke).toHaveBeenCalledWith("tick_n", { n: 10 });
    expect(useDebugStore.getState().currentTick).toBe(10);
  });
});

describe("commands — seekTick", () => {
  it("calls invoke('seek_tick') and applies seeked state", async () => {
    // First advance to set maxTick.
    useDebugStore.getState().applyTickResult({
      tick: 10,
      traceEvents: [],
      stateChanges: [],
    });

    const mockInvoke: TauriInvoke = vi.fn().mockResolvedValue({
      tick: 3,
      sm_states: [{ sm_id: 1, active_state: 0 }],
    });

    const cmds = createCommands(mockInvoke);
    await cmds.seekTick(3);

    expect(mockInvoke).toHaveBeenCalledWith("seek_tick", { tick: 3 });
    expect(useDebugStore.getState().currentTick).toBe(3);
    expect(useDebugStore.getState().maxTick).toBe(10); // unchanged
  });
});

describe("commands — getTopology", () => {
  it("calls invoke('get_topology') and stores result", async () => {
    const mockTopo = {
      nodes: [{ sm_id: { inner: 1 }, active_state: { inner: 0 }, label: "SM(1)" }],
      edges: [],
    };

    const mockInvoke: TauriInvoke = vi.fn().mockResolvedValue(mockTopo);

    const cmds = createCommands(mockInvoke);
    await cmds.getTopology();

    expect(mockInvoke).toHaveBeenCalledWith("get_topology");
    expect(useDebugStore.getState().topology).toEqual(mockTopo);
  });
});
