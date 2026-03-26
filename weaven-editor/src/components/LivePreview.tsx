import { useState, useCallback } from "react";
import { useEditorStore } from "../stores/editorStore";

interface WeavenAdapterLike {
  tick(): { smId: number; prev: number; next: number }[];
  activeState(smId: number): number;
  get smIds(): number[];
  get currentTick(): number;
}

interface Props {
  adapter: WeavenAdapterLike | null;
}

export default function LivePreview({ adapter }: Props) {
  const schema = useEditorStore((s) => s.schema);
  const [, forceUpdate] = useState(0);

  const handleTick = useCallback(() => {
    if (!adapter) return;
    adapter.tick();
    forceUpdate((n) => n + 1);
  }, [adapter]);

  if (!adapter || schema.state_machines.length === 0) {
    return (
      <div className="p-4 text-gray-500">
        Load a schema to preview simulation.
      </div>
    );
  }

  return (
    <div className="p-4 flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <button
          onClick={handleTick}
          className="px-3 py-1 text-xs rounded bg-indigo-600 hover:bg-indigo-500 text-white font-medium"
        >
          Tick
        </button>
        <span className="text-xs text-gray-400">
          Tick: {adapter.currentTick}
        </span>
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
    </div>
  );
}
