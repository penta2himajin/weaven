import type { WeavenAdapterLike, StateTransition } from "./LivePreview";

/**
 * Bridge class that wraps the weaven-wasm WeavenSession for use in LivePreview.
 * This adapts the WASM module's interface to WeavenAdapterLike.
 */
export class WasmAdapterBridge implements WeavenAdapterLike {
  private session: WasmSession | null = null;
  private _currentTick = 0;
  private _smIds: number[] = [];

  constructor(private wasmModule: WasmModule) {}

  loadSchema(json: string): void {
    this.session = this.wasmModule.WeavenSession.from_json(json);
    this._currentTick = 0;
    this._smIds = this.session.sm_ids();
  }

  tick(): StateTransition[] {
    if (!this.session) return [];
    const resultJson = this.session.tick_json();
    this._currentTick++;
    return this.parseTransitions(resultJson);
  }

  tickN(n: number): StateTransition[] {
    if (!this.session) return [];
    const resultJson = this.session.tick_n_json(n);
    this._currentTick += n;
    return this.parseTransitions(resultJson);
  }

  activeState(smId: number): number {
    if (!this.session) return 0;
    return this.session.active_state(smId);
  }

  snapshot(): string {
    if (!this.session) return "{}";
    return this.session.snapshot_json();
  }

  restore(snapshot: string): void {
    if (!this.session) return;
    this.session.restore_json(snapshot);
    this._currentTick = 0;
  }

  get smIds(): number[] {
    return this._smIds;
  }

  get currentTick(): number {
    return this._currentTick;
  }

  private parseTransitions(json: string): StateTransition[] {
    try {
      const result = JSON.parse(json);
      if (result.state_changes && Array.isArray(result.state_changes)) {
        return result.state_changes.map((sc: { sm_id: number; prev_state: number; new_state: number }) => ({
          smId: sc.sm_id,
          prev: sc.prev_state,
          next: sc.new_state,
        }));
      }
      return [];
    } catch {
      return [];
    }
  }
}

// Interfaces matching the weaven-wasm bindings
interface WasmSession {
  tick_json(): string;
  tick_n_json(n: number): string;
  active_state(smId: number): number;
  snapshot_json(): string;
  restore_json(snapshot: string): void;
  sm_ids(): number[];
}

interface WasmModule {
  WeavenSession: {
    from_json(json: string): WasmSession;
  };
}

/**
 * Attempt to load the WASM module and create a bridge adapter.
 * Returns null if WASM is not available.
 */
export async function createWasmAdapter(): Promise<WeavenAdapterLike | null> {
  try {
    // Dynamic import to avoid build failure when WASM is not available.
    // Use a variable to prevent Vite from statically analyzing the import.
    const modulePath = "weaven-wasm";
    const wasm = await (Function(`return import("${modulePath}")`)() as Promise<WasmModule>);
    return new WasmAdapterBridge(wasm);
  } catch {
    // WASM module not available - that's OK, LivePreview will show placeholder
    return null;
  }
}
