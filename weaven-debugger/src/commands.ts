/**
 * Command dispatch layer — thin abstraction between UI and Tauri IPC.
 *
 * `createCommands(invoke)` accepts an invoke function (real Tauri or mock)
 * and returns typed command wrappers that update the store.
 *
 * Rust backend returns snake_case; this layer normalizes to camelCase
 * for the store.
 */

import { useDebugStore } from "./stores/debugStore";
import type { TraceEvent } from "./generated/models";

/** Tauri invoke signature (mockable). */
export type TauriInvoke = (cmd: string, args?: Record<string, unknown>) => Promise<any>;

/** Normalize snake_case Rust response to camelCase store types. */
function normalizeTickResult(raw: any) {
  return {
    tick: raw.tick,
    traceEvents: (raw.trace_events ?? raw.traceEvents ?? []).map(normalizeTraceEvent),
    stateChanges: (raw.state_changes ?? raw.stateChanges ?? []).map((sc: any) => ({
      smId: normalizeId(sc.sm_id ?? sc.smId),
      fromState: normalizeId(sc.from_state ?? sc.fromState),
      toState: normalizeId(sc.to_state ?? sc.toState),
    })),
  };
}

function normalizeWorldState(raw: any) {
  return {
    tick: raw.tick,
    smStates: (raw.sm_states ?? raw.smStates ?? []).map((s: any) => ({
      smId: normalizeId(s.sm_id ?? s.smId),
      activeState: normalizeId(s.active_state ?? s.activeState),
    })),
  };
}

function normalizeId(v: any): { inner: number } {
  if (typeof v === "number") return { inner: v };
  if (v && typeof v.inner === "number") return v;
  return { inner: 0 };
}

/**
 * Normalize a Rust serde externally-tagged enum into a flat object with `kind`.
 *
 * Rust default:  { "GuardEvaluated": { tick: 1, sm_id: 1, ... } }
 * Frontend wants: { kind: "GuardEvaluated", tick: 1, smId: {...}, ... }
 */
function normalizeTraceEvent(raw: any): TraceEvent {
  // If already has `kind`, pass through (e.g. from tests).
  if (raw && raw.kind) return raw;

  // Externally tagged: single key is the variant name.
  const keys = Object.keys(raw);
  if (keys.length === 1) {
    const kind = keys[0];
    const inner = raw[kind];
    return {
      kind,
      tick: inner.tick,
      phase: inner.phase,
      // GuardEvaluated
      ...(inner.transition != null && { transition: normalizeId(inner.transition) }),
      ...(inner.sm_id != null && { smId: normalizeId(inner.sm_id) }),
      ...(inner.result != null && { result: inner.result }),
      ...(inner.context_snapshot != null && { contextSnapshot: inner.context_snapshot }),
      // TransitionFired
      ...(inner.from_state != null && { fromState: normalizeId(inner.from_state) }),
      ...(inner.to_state != null && { toState: normalizeId(inner.to_state) }),
      // SignalEmitted
      ...(inner.port != null && { port: normalizeId(inner.port) }),
      ...(inner.target != null && { target: normalizeId(inner.target) }),
      // CascadeStep
      ...(inner.depth != null && { depth: inner.depth }),
      ...(inner.queue_size != null && { queueSize: inner.queue_size }),
      // IrMatched
      ...(inner.rule_index != null && { ruleId: normalizeId(inner.rule_index) }),
      ...(inner.participants != null && { participants: new Set(inner.participants.map(normalizeId)) }),
      // PipelineFiltered
      ...(inner.connection != null && { connection: normalizeId(inner.connection) }),
    } as any;
  }

  return raw;
}

export function createCommands(invoke: TauriInvoke) {
  return {
    async tick() {
      const raw = await invoke("tick");
      const result = normalizeTickResult(raw);
      useDebugStore.getState().applyTickResult(result);
      return result;
    },

    async tickN(n: number) {
      const raw = await invoke("tick_n", { n });
      const result = normalizeTickResult(raw);
      useDebugStore.getState().applyTickResult(result);
      return result;
    },

    async seekTick(tick: number) {
      const raw = await invoke("seek_tick", { tick });
      const state = normalizeWorldState(raw);
      useDebugStore.getState().applySeeked(state);
      return state;
    },

    async getTopology() {
      const raw = await invoke("get_topology");
      useDebugStore.getState().setTopology(raw);
      return raw;
    },

    async loadSchema(path: string) {
      const raw = await invoke("load_schema", { path });
      useDebugStore.getState().setTopology(raw);
      return raw;
    },

    async injectSignal(smId: number, portId: number, payload: Record<string, number>) {
      await invoke("inject_signal", {
        smId,
        portId,
        payload,
      });
    },
  };
}
