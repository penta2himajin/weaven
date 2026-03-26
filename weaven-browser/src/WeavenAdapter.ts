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
