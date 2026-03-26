/**
 * Weaven Browser Adapter — TypeScript integration layer (§12.5, Phase 4).
 *
 * Bridges the Weaven WASM module to browser game loops (requestAnimationFrame,
 * Phaser scenes, custom Canvas renderers).
 *
 * Usage:
 * ```typescript
 * import { WeavenAdapter } from "@weaven/browser";
 *
 * const adapter = new WeavenAdapter();
 * await adapter.init("/path/to/weaven_wasm_bg.wasm");
 * await adapter.loadSchema(schemaJson);
 *
 * adapter.onStateChange((changes) => {
 *   for (const { smId, prev, next } of changes) {
 *     renderSm(smId, next);
 *   }
 * });
 *
 * function gameLoop() {
 *   adapter.pushInput(playerSmId, "trigger", inputValue);
 *   adapter.tick();
 *   requestAnimationFrame(gameLoop);
 * }
 * requestAnimationFrame(gameLoop);
 * ```
 */

// ---------------------------------------------------------------------------
// Types mirroring Rust structs (generated from weaven-debugger.als / models.ts)
// ---------------------------------------------------------------------------

/** Numeric SM identifier. */
export type SmId = number;

/** Numeric State identifier. */
export type StateId = number;

/** A state transition event emitted by tick(). */
export interface StateChange {
  readonly smId:   SmId;
  readonly prev:   StateId;
  readonly next:   StateId;
}

/** Callback invoked after each tick with the list of state changes. */
export type StateChangeHandler = (changes: StateChange[]) => void;

/** Callback invoked each tick regardless of state changes (for animations etc). */
export type TickHandler = (tick: number) => void;

/** A diff between two snapshots for a single SM. */
export interface SmStateDiff {
  readonly sm_id:           number;
  readonly prev_state:      number;
  readonly new_state:       number;
  readonly context_changes: Record<string, number>;
}

/** Authority model for networked SMs (§8.1). */
export type Authority = "Server" | "Owner" | "Local";

/** Sync policy for networked SMs (§8.2). */
export type SyncPolicy =
  | "InputSync"
  | "StateSync"
  | "None"
  | { ContextSync: { fields: string[] } };

/** Reconciliation policy for networked SMs (§8.3). */
export type ReconciliationPolicy =
  | "Snap"
  | "Rewind"
  | { Interpolate: { blend_ticks: number } };

/** Network policy declaration for an SM. */
export interface NetworkPolicy {
  readonly sm_id:          SmId;
  readonly authority:      Authority;
  readonly sync_policy:    SyncPolicy;
  readonly reconciliation: ReconciliationPolicy;
}

/** A tagged input for the rollback input buffer. */
export interface TaggedInput {
  readonly tick:        number;
  readonly target_sm:   SmId;
  readonly target_port: number;
  readonly payload:     Record<string, number>;
}

// ---------------------------------------------------------------------------
// WASM module interface (matches WeavenSession exported from weaven-wasm)
// ---------------------------------------------------------------------------

/** Shape of the wasm-bindgen exported WeavenSession class. */
interface WeavenSessionWasm {
  new(): WeavenSessionWasm;
  load_schema(json: string): void;
  enable_spatial(cellSize: number): void;
  tick(): string;                               // returns JSON StateChange[]
  push_input(smId: number, field: string, value: number): void;
  read_output(smId: number, field: string): number;
  active_state(smId: number): number;
  inject_signal(smId: number, portId: number, signalType: number, payloadJson: string): void;
  activate(smId: number): void;
  set_position(smId: number, x: number, y: number): void;
  snapshot_json(): string;
  restore_json(json: string): void;
  current_tick(): bigint;
  sm_ids_json(): string;
  // Network APIs (§8)
  diff_snapshots_json(beforeJson: string, afterJson: string): string;
  set_network_policy(policyJson: string): void;
  policy_filtered_diff_json(diffsJson: string): string;
  scoped_snapshot_json(smIdsJson: string): string;
  interest_region_json(cx: number, cy: number, radius: number): string;
  init_input_buffer(historyDepth: number): void;
  push_tagged_input(inputJson: string): void;
  apply_buffered_inputs(): void;
  save_rewind_base(): void;
  rewind_to(targetTick: bigint, currentTick: bigint): void;
  free(): void;
}

// ---------------------------------------------------------------------------
// WeavenAdapter
// ---------------------------------------------------------------------------

/**
 * Browser-side adapter for Weaven Core WASM.
 *
 * Wraps WeavenSession with:
 * - async init/load lifecycle
 * - requestAnimationFrame integration helpers
 * - typed callback registration
 * - HitStop / SlowMotion support
 */
export class WeavenAdapter {
  private session: WeavenSessionWasm | null = null;
  private stateChangeHandlers: StateChangeHandler[] = [];
  private tickHandlers: TickHandler[] = [];
  private running = false;
  private rafHandle: number | null = null;
  private hitStopRemaining = 0;
  private slowMotionFactor = 1.0;
  private slowMotionRemaining = 0;

  // ── Init ────────────────────────────────────────────────────────────────

  /**
   * Initialise the WASM module and create a WeavenSession.
   *
   * @param wasmUrl  URL or path to `weaven_wasm_bg.wasm`.
   *                 If omitted, assumes wasm-bindgen inline init has been called.
   */
  async init(
    wasmInit: (() => Promise<{ WeavenSession: new () => WeavenSessionWasm }>) | null = null,
  ): Promise<void> {
    if (wasmInit) {
      const wasm = await wasmInit();
      this.session = new wasm.WeavenSession();
    } else {
      // Caller is responsible for calling the wasm-bindgen init() before this.
      // WeavenSession must be available on globalThis._weavenWasm.
      const g = globalThis as any;
      if (!g._weavenWasm?.WeavenSession) {
        throw new Error("WeavenSession not found. Call wasm-bindgen init() first.");
      }
      this.session = new g._weavenWasm.WeavenSession();
    }
  }

  /** Load SM definitions from a Weaven Schema JSON string. */
  loadSchema(json: string): void {
    this.assertSession();
    this.session!.load_schema(json);
  }

  /** Enable the Tier 2 spatial index with the given cell size. */
  enableSpatial(cellSize: number): void {
    this.assertSession();
    this.session!.enable_spatial(cellSize);
  }

  // ── Tick ─────────────────────────────────────────────────────────────────

  /**
   * Advance the simulation by one tick.
   *
   * Respects HitStop (skips tick) and SlowMotion (may skip based on factor).
   * Returns the list of state changes produced this tick.
   */
  tick(): StateChange[] {
    this.assertSession();

    // HitStop: freeze simulation
    if (this.hitStopRemaining > 0) {
      this.hitStopRemaining--;
      this._fireTickHandlers();
      return [];
    }

    // SlowMotion: skip ticks based on factor (e.g. factor=0.5 → tick every 2 frames)
    if (this.slowMotionRemaining > 0) {
      this.slowMotionRemaining--;
      // Probabilistic skip based on factor (deterministic via frame counter would be better in production)
      if (Math.random() > this.slowMotionFactor) {
        this._fireTickHandlers();
        return [];
      }
    }

    const changesJson = this.session!.tick();
    const changes: StateChange[] = JSON.parse(changesJson).map((c: any) => ({
      smId: c.sm_id,
      prev: c.prev,
      next: c.next,
    }));

    if (changes.length > 0) {
      this.stateChangeHandlers.forEach((h) => h(changes));
    }
    this._fireTickHandlers();
    return changes;
  }

  /** Apply HitStop — freeze simulation for `frames` frames. */
  applyHitStop(frames: number): void {
    this.hitStopRemaining = Math.max(this.hitStopRemaining, frames);
  }

  /** Apply SlowMotion — reduce tick rate for `durationTicks` ticks. */
  applySlowMotion(factor: number, durationTicks: number): void {
    this.slowMotionFactor = factor;
    this.slowMotionRemaining = durationTicks;
  }

  // ── requestAnimationFrame loop ───────────────────────────────────────────

  /** Start the game loop (calls tick() every animation frame). */
  startLoop(): void {
    if (this.running) return;
    this.running = true;
    const loop = () => {
      if (!this.running) return;
      this.tick();
      this.rafHandle = requestAnimationFrame(loop);
    };
    this.rafHandle = requestAnimationFrame(loop);
  }

  /** Stop the game loop. */
  stopLoop(): void {
    this.running = false;
    if (this.rafHandle !== null) {
      cancelAnimationFrame(this.rafHandle);
      this.rafHandle = null;
    }
  }

  // ── Port I/O ──────────────────────────────────────────────────────────────

  /** Push a continuous input value to an SM's context field (§2.4.3). */
  pushInput(smId: SmId, field: string, value: number): void {
    this.assertSession();
    this.session!.push_input(smId, field, value);
  }

  /** Read a context field from an SM's output (§2.4.4). */
  readOutput(smId: SmId, field: string): number {
    this.assertSession();
    return this.session!.read_output(smId, field);
  }

  /** Get the active state ID for an SM. Returns -1 if SM not found. */
  activeState(smId: SmId): StateId {
    this.assertSession();
    return this.session!.active_state(smId);
  }

  /** Inject a discrete signal into an SM's Input Port. */
  injectSignal(
    smId: SmId,
    portId: number,
    signalType: number,
    payload: Record<string, number>,
  ): void {
    this.assertSession();
    this.session!.inject_signal(smId, portId, signalType, JSON.stringify(payload));
  }

  /** Wake an SM so it will be evaluated on the next tick. */
  activate(smId: SmId): void {
    this.assertSession();
    this.session!.activate(smId);
  }

  // ── Spatial ───────────────────────────────────────────────────────────────

  /** Update an SM's world position (Tier 2). */
  setPosition(smId: SmId, x: number, y: number): void {
    this.assertSession();
    this.session!.set_position(smId, x, y);
  }

  // ── Callbacks ─────────────────────────────────────────────────────────────

  /** Register a callback invoked with state changes after each tick. */
  onStateChange(handler: StateChangeHandler): () => void {
    this.stateChangeHandlers.push(handler);
    return () => {
      this.stateChangeHandlers = this.stateChangeHandlers.filter((h) => h !== handler);
    };
  }

  /** Register a callback invoked every tick (even with no state changes). */
  onTick(handler: TickHandler): () => void {
    this.tickHandlers.push(handler);
    return () => {
      this.tickHandlers = this.tickHandlers.filter((h) => h !== handler);
    };
  }

  // ── Snapshot / Restore ───────────────────────────────────────────────────

  /** Take a serializable snapshot of the current world state. */
  takeSnapshot(): string {
    this.assertSession();
    return this.session!.snapshot_json();
  }

  /** Restore world state from a previously taken snapshot. */
  restoreSnapshot(json: string): void {
    this.assertSession();
    this.session!.restore_json(json);
  }

  /** Current simulation tick number. */
  get currentTick(): number {
    this.assertSession();
    return Number(this.session!.current_tick());
  }

  /** All registered SM IDs. */
  get smIds(): SmId[] {
    this.assertSession();
    return JSON.parse(this.session!.sm_ids_json());
  }

  // ── Network APIs (§8) ──────────────────────────────────────────────────────

  /**
   * Compute the diff between two snapshots (before/after a tick).
   * Returns the list of SM state diffs.
   */
  diffSnapshots(beforeJson: string, afterJson: string): SmStateDiff[] {
    this.assertSession();
    return JSON.parse(this.session!.diff_snapshots_json(beforeJson, afterJson));
  }

  /**
   * Register a network policy for an SM.
   * Controls sync behaviour, authority, and reconciliation strategy.
   */
  setNetworkPolicy(policy: NetworkPolicy): void {
    this.assertSession();
    this.session!.set_network_policy(JSON.stringify(policy));
  }

  /**
   * Filter a diff list by registered network policies.
   * SMs with SyncPolicy "None" or "InputSync" are excluded.
   */
  policyFilteredDiff(diffs: SmStateDiff[]): SmStateDiff[] {
    this.assertSession();
    return JSON.parse(this.session!.policy_filtered_diff_json(JSON.stringify(diffs)));
  }

  /**
   * Take a scoped snapshot — only the listed SM IDs.
   * Useful for interest region / fog-of-war snapshots.
   */
  scopedSnapshot(smIds: SmId[]): string {
    this.assertSession();
    return this.session!.scoped_snapshot_json(JSON.stringify(smIds));
  }

  /**
   * Return SM IDs within a spatial radius (interest region management).
   */
  interestRegion(cx: number, cy: number, radius: number): SmId[] {
    this.assertSession();
    return JSON.parse(this.session!.interest_region_json(cx, cy, radius));
  }

  /**
   * Initialise the input buffer for rollback networking.
   * @param historyDepth  Number of ticks to retain in the buffer.
   */
  initInputBuffer(historyDepth: number): void {
    this.assertSession();
    this.session!.init_input_buffer(historyDepth);
  }

  /**
   * Push a tagged input into the rollback input buffer.
   */
  pushTaggedInput(input: TaggedInput): void {
    this.assertSession();
    this.session!.push_tagged_input(JSON.stringify(input));
  }

  /**
   * Apply buffered inputs for the current tick to the world.
   */
  applyBufferedInputs(): void {
    this.assertSession();
    this.session!.apply_buffered_inputs();
  }

  /**
   * Save the current world state as the rewind base point.
   */
  saveRewindBase(): void {
    this.assertSession();
    this.session!.save_rewind_base();
  }

  /**
   * Rewind to the saved base snapshot and re-simulate to `currentTick`,
   * replaying all buffered inputs.
   */
  rewindTo(targetTick: number, currentTick: number): void {
    this.assertSession();
    this.session!.rewind_to(BigInt(targetTick), BigInt(currentTick));
  }

  // ── Cleanup ───────────────────────────────────────────────────────────────

  /** Free the WASM session and stop the loop. */
  dispose(): void {
    this.stopLoop();
    this.session?.free();
    this.session = null;
  }

  // ── Private ───────────────────────────────────────────────────────────────

  private assertSession(): void {
    if (!this.session) {
      throw new Error("WeavenAdapter not initialised. Call init() first.");
    }
  }

  private _fireTickHandlers(): void {
    if (this.tickHandlers.length === 0) return;
    const tick = this.session ? Number(this.session.current_tick()) : 0;
    this.tickHandlers.forEach((h) => h(tick));
  }
}
