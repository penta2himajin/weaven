/**
 * WeavenAdapter unit tests.
 * These tests mock the WASM session to verify adapter logic without WASM.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { WeavenAdapter } from "../WeavenAdapter";
import type { StateChange } from "../WeavenAdapter";

// ---------------------------------------------------------------------------
// Mock WeavenSession (simulates the wasm-bindgen exported class)
// ---------------------------------------------------------------------------

function makeMockSession() {
  const state: Record<number, number> = { 1: 0, 2: 0 };
  const context: Record<string, number> = {};
  let tick = 0;

  return {
    load_schema: vi.fn(),
    enable_spatial: vi.fn(),
    tick: vi.fn(() => {
      tick++;
      // Simulate SM 1 transitioning S0→S1 when trigger > 0
      const changes: { sm_id: number; prev: number; next: number }[] = [];
      if ((context["trigger"] ?? 0) > 0 && state[1] === 0) {
        state[1] = 1;
        changes.push({ sm_id: 1, prev: 0, next: 1 });
      }
      return JSON.stringify(changes);
    }),
    push_input: vi.fn((smId: number, field: string, value: number) => {
      context[field] = value;
    }),
    read_output: vi.fn((smId: number, field: string) => context[field] ?? 0),
    active_state: vi.fn((smId: number) => state[smId] ?? -1),
    inject_signal: vi.fn(),
    activate: vi.fn(),
    set_position: vi.fn(),
    snapshot_json: vi.fn(() => JSON.stringify({ tick, state, context })),
    restore_json: vi.fn(),
    current_tick: vi.fn(() => BigInt(tick)),
    sm_ids_json: vi.fn(() => JSON.stringify([1, 2])),
    free: vi.fn(),
    _state: state,
    _context: context,
    _tick: () => tick,
  };
}

/** Inject a mock session directly into an adapter (bypasses WASM init). */
function adapterWithMock(mock: ReturnType<typeof makeMockSession>): WeavenAdapter {
  const adapter = new WeavenAdapter();
  (adapter as any).session = mock;
  return adapter;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("WeavenAdapter — init guard", () => {
  it("throws before init()", () => {
    const adapter = new WeavenAdapter();
    expect(() => adapter.tick()).toThrow(/not initialised/i);
  });

  it("dispose() frees session and clears state", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    adapter.dispose();
    expect(mock.free).toHaveBeenCalledOnce();
    expect(() => adapter.tick()).toThrow(/not initialised/i);
  });
});

describe("WeavenAdapter — tick", () => {
  it("parses state changes from JSON", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    mock._context["trigger"] = 1;
    const changes = adapter.tick();
    expect(changes).toHaveLength(1);
    expect(changes[0]).toMatchObject<StateChange>({ smId: 1, prev: 0, next: 1 });
  });

  it("returns empty array when no transitions fire", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    const changes = adapter.tick();
    expect(changes).toHaveLength(0);
  });

  it("fires onStateChange callback", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    mock._context["trigger"] = 1;

    const received: StateChange[][] = [];
    adapter.onStateChange((changes) => received.push(changes));

    adapter.tick();
    expect(received).toHaveLength(1);
    expect(received[0][0].smId).toBe(1);
  });

  it("fires onTick callback every tick regardless of changes", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    const ticks: number[] = [];
    adapter.onTick((t) => ticks.push(t));
    adapter.tick();
    adapter.tick();
    expect(ticks).toHaveLength(2);
  });

  it("onStateChange unsubscribe works", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    mock._context["trigger"] = 1;

    const received: StateChange[][] = [];
    const unsub = adapter.onStateChange((changes) => received.push(changes));
    adapter.tick();
    unsub();
    // Reset and tick again
    mock._state[1] = 0;
    mock._context["trigger"] = 1;
    adapter.tick();
    // Still only 1 call (before unsub)
    expect(received).toHaveLength(1);
  });
});

describe("WeavenAdapter — HitStop", () => {
  it("skips tick simulation during hit stop frames", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    adapter.applyHitStop(2);

    // Frames 1 and 2: frozen
    adapter.tick();
    adapter.tick();
    expect(mock.tick).not.toHaveBeenCalled();

    // Frame 3: resumes
    adapter.tick();
    expect(mock.tick).toHaveBeenCalledOnce();
  });

  it("max(current, new) is applied for overlapping hit stops", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    adapter.applyHitStop(1);
    adapter.applyHitStop(3); // 3 > 1 → should use 3
    let callCount = 0;
    mock.tick.mockImplementation(() => { callCount++; return "[]"; });
    adapter.tick(); adapter.tick(); adapter.tick(); // 3 frozen frames
    expect(callCount).toBe(0);
    adapter.tick(); // frame 4 — resumes
    expect(callCount).toBe(1);
  });
});

describe("WeavenAdapter — port I/O", () => {
  it("pushInput delegates to session", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    adapter.pushInput(1, "speed", 5.0);
    expect(mock.push_input).toHaveBeenCalledWith(1, "speed", 5.0);
  });

  it("readOutput delegates to session", () => {
    const mock = makeMockSession();
    mock._context["hp"] = 80;
    const adapter = adapterWithMock(mock);
    expect(adapter.readOutput(1, "hp")).toBe(80);
  });

  it("activeState returns correct state", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    expect(adapter.activeState(1)).toBe(0);
  });

  it("injectSignal serialises payload to JSON", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    adapter.injectSignal(1, 0, 0, { intensity: 5.0 });
    expect(mock.inject_signal).toHaveBeenCalledWith(
      1, 0, 0, JSON.stringify({ intensity: 5.0 }),
    );
  });
});

describe("WeavenAdapter — spatial", () => {
  it("setPosition delegates to session", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    adapter.setPosition(1, 3.0, 4.0);
    expect(mock.set_position).toHaveBeenCalledWith(1, 3.0, 4.0);
  });
});

describe("WeavenAdapter — snapshot", () => {
  it("takeSnapshot returns JSON string", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    const snap = adapter.takeSnapshot();
    expect(typeof snap).toBe("string");
    expect(() => JSON.parse(snap)).not.toThrow();
  });

  it("restoreSnapshot delegates to session", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    adapter.restoreSnapshot('{"tick":0}');
    expect(mock.restore_json).toHaveBeenCalledWith('{"tick":0}');
  });
});

describe("WeavenAdapter — smIds + currentTick", () => {
  it("smIds returns parsed array", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    expect(adapter.smIds).toEqual([1, 2]);
  });

  it("currentTick converts BigInt to number", () => {
    const mock = makeMockSession();
    const adapter = adapterWithMock(mock);
    adapter.tick(); // tick = 1
    expect(adapter.currentTick).toBe(1);
  });
});
