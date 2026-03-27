import { describe, it, expect, vi } from "vitest";
import { WasmAdapterBridge } from "../components/WasmAdapterBridge";

function createMockWasm() {
  const session = {
    tick_json: vi.fn(() => '{"state_changes":[{"sm_id":1,"prev_state":0,"new_state":1}]}'),
    tick_n_json: vi.fn(() => '{"state_changes":[]}'),
    active_state: vi.fn((smId: number) => smId === 1 ? 1 : 0),
    snapshot_json: vi.fn(() => '{"snapshot":"data"}'),
    restore_json: vi.fn(),
    sm_ids: vi.fn(() => [1, 2]),
  };
  return {
    module: {
      WeavenSession: {
        from_json: vi.fn(() => session),
      },
    },
    session,
  };
}

describe("WasmAdapterBridge", () => {
  it("loadSchema creates a new session", () => {
    const { module } = createMockWasm();
    const bridge = new WasmAdapterBridge(module);
    bridge.loadSchema('{"state_machines":[]}');
    expect(module.WeavenSession.from_json).toHaveBeenCalledWith('{"state_machines":[]}');
  });

  it("tick delegates to session and increments tick count", () => {
    const { module, session } = createMockWasm();
    const bridge = new WasmAdapterBridge(module);
    bridge.loadSchema("{}");

    const transitions = bridge.tick();
    expect(session.tick_json).toHaveBeenCalled();
    expect(bridge.currentTick).toBe(1);
    expect(transitions).toEqual([{ smId: 1, prev: 0, next: 1 }]);
  });

  it("tickN delegates to session and increments tick count by n", () => {
    const { module, session } = createMockWasm();
    const bridge = new WasmAdapterBridge(module);
    bridge.loadSchema("{}");

    bridge.tickN(10);
    expect(session.tick_n_json).toHaveBeenCalledWith(10);
    expect(bridge.currentTick).toBe(10);
  });

  it("activeState delegates to session", () => {
    const { module } = createMockWasm();
    const bridge = new WasmAdapterBridge(module);
    bridge.loadSchema("{}");

    expect(bridge.activeState(1)).toBe(1);
  });

  it("snapshot delegates to session", () => {
    const { module, session } = createMockWasm();
    const bridge = new WasmAdapterBridge(module);
    bridge.loadSchema("{}");

    expect(bridge.snapshot()).toBe('{"snapshot":"data"}');
    expect(session.snapshot_json).toHaveBeenCalled();
  });

  it("restore delegates to session and resets tick", () => {
    const { module, session } = createMockWasm();
    const bridge = new WasmAdapterBridge(module);
    bridge.loadSchema("{}");
    bridge.tick();
    expect(bridge.currentTick).toBe(1);

    bridge.restore('{"snapshot":"data"}');
    expect(session.restore_json).toHaveBeenCalledWith('{"snapshot":"data"}');
    expect(bridge.currentTick).toBe(0);
  });

  it("smIds returns session sm_ids", () => {
    const { module } = createMockWasm();
    const bridge = new WasmAdapterBridge(module);
    bridge.loadSchema("{}");
    expect(bridge.smIds).toEqual([1, 2]);
  });

  it("returns empty array when no session loaded", () => {
    const { module } = createMockWasm();
    const bridge = new WasmAdapterBridge(module);
    expect(bridge.tick()).toEqual([]);
    expect(bridge.smIds).toEqual([]);
  });
});
