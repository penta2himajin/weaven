import { create } from "zustand";
import type { TraceEvent, SmStateDiff } from "../generated/models";

/** Lightweight SM ID wrapper matching Rust serialization. */
export interface SmId {
  inner: number;
}

/** Mirrors Rust TickResult from Tauri commands. */
export interface TickResult {
  tick: number;
  traceEvents: TraceEvent[];
  stateChanges: { smId: SmId; fromState: { inner: number }; toState: { inner: number } }[];
  diffs: SmStateDiff[];
}

/** Mirrors Rust WorldState from Tauri commands. */
export interface WorldState {
  tick: number;
  smStates: { smId: SmId; activeState: { inner: number } }[];
}

/** Topology types matching Rust serialization. */
export interface GraphNode {
  sm_id: SmId;
  active_state: { inner: number } | null;
  label: string;
}

export interface GraphEdge {
  source: SmId;
  target: SmId;
  kind: string;
  connection_id: { inner: number } | null;
  label: string;
}

export interface Topology {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

interface DebugStore {
  // State
  loaded: boolean;
  currentTick: number;
  maxTick: number;
  topology: Topology | null;
  traceEvents: TraceEvent[];
  selectedSmId: SmId | null;
  cascadeIndex: number;
  selectedTraceIndex: number | null;
  filterConfig: FilterConfig;
  diffs: SmStateDiff[];

  // Actions
  setTopology: (topology: Topology) => void;
  selectSm: (smId: SmId | null) => void;
  applyTickResult: (result: TickResult) => void;
  applySeeked: (state: WorldState) => void;
  selectTraceEvent: (index: number) => void;
  clearTraceSelection: () => void;
  toggleSmVisibility: (smId: number) => void;
  togglePhaseVisibility: (phase: string) => void;
  resetFilter: () => void;

  // Derived (computed as functions)
  filteredTraceEvents: () => TraceEvent[];
  cascadeSteps: () => TraceEvent[];
  nextCascadeStep: () => void;
  prevCascadeStep: () => void;
  highlightedEdges: () => HighlightedEdge[];
  signalsForCascadeStep: (stepIndex: number) => TraceEvent[];
  diffForSm: (smId: number) => SmStateDiff | undefined;
  changedSmIds: () => number[];
}

export interface FilterConfig {
  hiddenSmIds: Set<number>;
  hiddenPhases: Set<string>;
}

export interface HighlightedEdge {
  source: number;
  target: number;
  kind: "signal" | "filtered";
}

export const useDebugStore = create<DebugStore>((set, get) => ({
  loaded: false,
  currentTick: 0,
  maxTick: 0,
  topology: null,
  traceEvents: [],
  selectedSmId: null,
  cascadeIndex: 0,
  selectedTraceIndex: null,
  filterConfig: { hiddenSmIds: new Set(), hiddenPhases: new Set() },
  diffs: [],

  setTopology: (topology) => set({ topology, loaded: true }),

  selectSm: (smId) => set({ selectedSmId: smId }),

  selectTraceEvent: (index) => set({ selectedTraceIndex: index }),

  clearTraceSelection: () => set({ selectedTraceIndex: null }),

  toggleSmVisibility: (smId) =>
    set((state) => {
      const next = new Set(state.filterConfig.hiddenSmIds);
      if (next.has(smId)) next.delete(smId);
      else next.add(smId);
      return { filterConfig: { ...state.filterConfig, hiddenSmIds: next } };
    }),

  togglePhaseVisibility: (phase) =>
    set((state) => {
      const next = new Set(state.filterConfig.hiddenPhases);
      if (next.has(phase)) next.delete(phase);
      else next.add(phase);
      return { filterConfig: { ...state.filterConfig, hiddenPhases: next } };
    }),

  resetFilter: () =>
    set({ filterConfig: { hiddenSmIds: new Set(), hiddenPhases: new Set() } }),

  applyTickResult: (result) =>
    set((state) => ({
      loaded: true,
      currentTick: result.tick,
      maxTick: Math.max(state.maxTick, result.tick),
      traceEvents: result.traceEvents,
      cascadeIndex: 0,
      diffs: result.diffs ?? [],
    })),

  applySeeked: (ws) =>
    set({
      currentTick: ws.tick,
      traceEvents: [],
      cascadeIndex: 0,
      diffs: [],
    }),

  filteredTraceEvents: () => {
    const { traceEvents, selectedSmId, filterConfig } = get();
    const { hiddenSmIds, hiddenPhases } = filterConfig;

    let result = traceEvents;

    // Apply phase filter.
    if (hiddenPhases.size > 0) {
      result = result.filter((e: any) => {
        const phase = e.phase;
        return !phase || !hiddenPhases.has(String(phase));
      });
    }

    // Apply hidden SM filter.
    if (hiddenSmIds.size > 0) {
      result = result.filter((e: any) => {
        if ("smId" in e && e.smId != null) {
          const id = e.smId.inner ?? e.smId;
          return !hiddenSmIds.has(id);
        }
        // SignalDelivered uses targetSm instead of smId
        if ("targetSm" in e && e.targetSm != null) {
          const id = e.targetSm.inner ?? e.targetSm;
          return !hiddenSmIds.has(id);
        }
        return true; // Keep events without smId (e.g. CascadeStep).
      });
    }

    // Apply selected SM filter (narrows further).
    if (selectedSmId) {
      result = result.filter((e: any) => {
        if ("smId" in e && e.smId != null) {
          return e.smId.inner === selectedSmId.inner;
        }
        if ("targetSm" in e && e.targetSm != null) {
          return e.targetSm.inner === selectedSmId.inner;
        }
        return true;
      });
    }

    return result;
  },

  cascadeSteps: () => {
    const { traceEvents } = get();
    return traceEvents.filter((e: any) => e.kind === "CascadeStep");
  },

  nextCascadeStep: () =>
    set((state) => {
      const steps = get().cascadeSteps();
      const maxIdx = steps.length - 1;
      return { cascadeIndex: Math.min(state.cascadeIndex + 1, Math.max(0, maxIdx)) };
    }),

  prevCascadeStep: () =>
    set((state) => ({
      cascadeIndex: Math.max(0, state.cascadeIndex - 1),
    })),

  highlightedEdges: () => {
    const { traceEvents, selectedTraceIndex } = get();
    if (selectedTraceIndex === null || selectedTraceIndex === undefined) return [];

    const event: any = traceEvents[selectedTraceIndex];
    if (!event) return [];

    const edges: HighlightedEdge[] = [];

    if (event.kind === "SignalEmitted" && event.target) {
      const source = event.smId?.inner ?? event.smId;
      const target = event.target?.inner ?? event.target;
      if (typeof source === "number" && typeof target === "number") {
        edges.push({ source, target, kind: "signal" });
      }
    } else if (event.kind === "SignalDelivered") {
      const source = event.sourceSm?.inner ?? event.sourceSm;
      const target = event.targetSm?.inner ?? event.targetSm;
      if (typeof source === "number" && typeof target === "number") {
        edges.push({ source, target, kind: "signal" });
      }
    } else if (event.kind === "PipelineFiltered") {
      const targetSm = event.smId?.inner ?? event.smId;
      if (typeof targetSm === "number") {
        edges.push({ source: targetSm, target: targetSm, kind: "filtered" });
      }
    }

    return edges;
  },

  /** Get SignalDelivered events for a specific cascade step index. */
  signalsForCascadeStep: (stepIndex: number) => {
    const { traceEvents } = get();
    const steps = traceEvents.filter((e: any) => e.kind === "CascadeStep");
    if (stepIndex < 0 || stepIndex >= steps.length) return [];
    const step: any = steps[stepIndex];
    const depth = step.depth;

    // Return all SignalDelivered events at this depth.
    return traceEvents.filter(
      (e: any) => e.kind === "SignalDelivered" && e.depth === depth,
    );
  },

  /** Get the diff for a specific SM (by raw id). */
  diffForSm: (smId: number) => {
    const { diffs } = get();
    return diffs.find((d) => d.smId === smId);
  },

  /** Get all SM IDs that changed this tick. */
  changedSmIds: () => {
    const { diffs } = get();
    return diffs.map((d) => d.smId);
  },
}));
