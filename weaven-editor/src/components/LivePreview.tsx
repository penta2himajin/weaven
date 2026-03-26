import { useState, useCallback, useEffect, useRef } from "react";
import { useEditorStore } from "../stores/editorStore";

export interface WeavenAdapterLike {
  loadSchema(json: string): void;
  tick(): StateTransition[];
  tickN(n: number): StateTransition[];
  activeState(smId: number): number;
  snapshot(): string;
  restore(snapshot: string): void;
  get smIds(): number[];
  get currentTick(): number;
}

export interface StateTransition {
  smId: number;
  prev: number;
  next: number;
}

interface Props {
  adapter: WeavenAdapterLike | null;
}

export default function LivePreview({ adapter }: Props) {
  const schema = useEditorStore((s) => s.schema);
  const exportJson = useEditorStore((s) => s.exportJson);
  const [, forceUpdate] = useState(0);
  const [transitions, setTransitions] = useState<StateTransition[]>([]);
  const [running, setRunning] = useState(false);
  const [tickRate, setTickRate] = useState(10);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const [savedSnapshot, setSavedSnapshot] = useState<string | null>(null);

  // Reload schema into adapter when schema changes
  useEffect(() => {
    if (!adapter) return;
    try {
      adapter.loadSchema(exportJson());
      setTransitions([]);
      forceUpdate((n) => n + 1);
    } catch {
      // adapter may not support loadSchema in all states
    }
  }, [adapter, schema, exportJson]);

  // Continuous simulation
  useEffect(() => {
    if (!running || !adapter) return;
    intervalRef.current = setInterval(() => {
      const t = adapter.tick();
      setTransitions(t);
      forceUpdate((n) => n + 1);
    }, 1000 / tickRate);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [running, adapter, tickRate]);

  const handleTick = useCallback(() => {
    if (!adapter) return;
    const t = adapter.tick();
    setTransitions(t);
    forceUpdate((n) => n + 1);
  }, [adapter]);

  const handleTickN = useCallback(() => {
    if (!adapter) return;
    const t = adapter.tickN(tickRate);
    setTransitions(t);
    forceUpdate((n) => n + 1);
  }, [adapter, tickRate]);

  const handleSnapshot = useCallback(() => {
    if (!adapter) return;
    setSavedSnapshot(adapter.snapshot());
  }, [adapter]);

  const handleRestore = useCallback(() => {
    if (!adapter || !savedSnapshot) return;
    adapter.restore(savedSnapshot);
    setTransitions([]);
    forceUpdate((n) => n + 1);
  }, [adapter, savedSnapshot]);

  if (!adapter || schema.state_machines.length === 0) {
    return (
      <div className="p-4 text-gray-500">
        Load a schema to preview simulation.
      </div>
    );
  }

  return (
    <div className="p-4 flex flex-col gap-3">
      <div className="flex items-center gap-2 flex-wrap">
        <button
          onClick={handleTick}
          className="px-3 py-1 text-xs rounded bg-indigo-600 hover:bg-indigo-500 text-white font-medium"
        >
          Tick
        </button>
        <button
          onClick={handleTickN}
          className="px-3 py-1 text-xs rounded bg-indigo-700 hover:bg-indigo-600 text-white font-medium"
        >
          Tick x{tickRate}
        </button>
        <button
          onClick={() => setRunning((r) => !r)}
          className={`px-3 py-1 text-xs rounded font-medium ${
            running
              ? "bg-red-600 hover:bg-red-500 text-white"
              : "bg-green-700 hover:bg-green-600 text-white"
          }`}
        >
          {running ? "Stop" : "Run"}
        </button>
        <span className="text-xs text-gray-400">
          Tick: {adapter.currentTick}
        </span>
      </div>

      <div className="flex items-center gap-2">
        <label className="text-xs text-gray-400">Rate:</label>
        <input
          type="number"
          min={1}
          max={60}
          value={tickRate}
          onChange={(e) => setTickRate(parseInt(e.target.value, 10) || 1)}
          className="w-14 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
          aria-label="tick rate"
        />
        <span className="text-xs text-gray-500">ticks/sec</span>
      </div>

      <div className="flex items-center gap-2">
        <button
          onClick={handleSnapshot}
          className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
        >
          Save Snapshot
        </button>
        <button
          onClick={handleRestore}
          disabled={!savedSnapshot}
          className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300 disabled:opacity-50"
        >
          Restore Snapshot
        </button>
      </div>

      <section>
        <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">SM States</h4>
        <ul className="space-y-1">
          {adapter.smIds.map((smId) => (
            <li key={smId} className="text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
              SM({smId}) — State: {adapter.activeState(smId)}
            </li>
          ))}
        </ul>
      </section>

      {transitions.length > 0 && (
        <section>
          <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">Last Transitions</h4>
          <ul className="space-y-1">
            {transitions.map((t, i) => (
              <li key={i} className="text-xs text-yellow-300 px-2 py-1 bg-gray-800 rounded">
                SM({t.smId}): {t.prev} → {t.next}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
