import { useDebugStore } from "../stores/debugStore";
import { useCommands } from "./CommandsContext";

export default function TimelinePanel() {
  const currentTick = useDebugStore((s) => s.currentTick);
  const maxTick = useDebugStore((s) => s.maxTick);
  const cascadeIndex = useDebugStore((s) => s.cascadeIndex);
  const nextCascade = useDebugStore((s) => s.nextCascadeStep);
  const prevCascade = useDebugStore((s) => s.prevCascadeStep);

  let cmds: ReturnType<typeof useCommands> | null = null;
  try {
    cmds = useCommands();
  } catch {
    // No provider — render without commands (static preview).
  }

  return (
    <div className="flex items-center gap-4 px-4 py-2 bg-gray-900 border-b border-gray-800 shrink-0">
      {/* Tick controls */}
      <div className="flex items-center gap-2">
        <button
          className="px-2 py-1 text-xs rounded bg-gray-800 hover:bg-gray-700 text-gray-300 transition-colors"
          aria-label="Step back"
          onClick={() => cmds?.seekTick(Math.max(0, currentTick - 1))}
        >
          ◀
        </button>
        <button
          className="px-3 py-1 text-xs rounded bg-indigo-600 hover:bg-indigo-500 text-white font-medium transition-colors"
          aria-label="Tick"
          onClick={() => cmds?.tick()}
        >
          Tick
        </button>
        <button
          className="px-2 py-1 text-xs rounded bg-gray-800 hover:bg-gray-700 text-gray-300 transition-colors"
          aria-label="Step forward"
          onClick={() => cmds?.tick()}
        >
          ▶
        </button>
        <button
          className="px-2 py-1 text-xs rounded bg-gray-800 hover:bg-gray-700 text-gray-300 transition-colors"
          aria-label="×10"
          onClick={() => cmds?.tickN(10)}
        >
          ×10
        </button>
      </div>

      {/* Tick slider */}
      <div className="flex-1 flex items-center gap-3">
        <input
          type="range"
          min={0}
          max={maxTick || 1}
          value={currentTick}
          onChange={(e) => cmds?.seekTick(Number(e.target.value))}
          className="flex-1 h-1 accent-indigo-500 bg-gray-700 rounded-lg cursor-pointer"
          aria-label="Tick slider"
        />
      </div>

      {/* Tick display */}
      <div className="text-xs font-mono text-gray-400 min-w-[100px] text-right" data-testid="tick-display">
        Tick <span className="text-gray-200">{currentTick}</span>
        <span className="text-gray-600"> / {maxTick}</span>
      </div>

      {/* Cascade stepper */}
      <div className="flex items-center gap-1 border-l border-gray-700 pl-3">
        <span className="text-[10px] text-gray-500 uppercase tracking-wider mr-1">
          Cascade
        </span>
        <button
          className="px-1.5 py-0.5 text-[10px] rounded bg-gray-800 hover:bg-gray-700 text-gray-400 transition-colors"
          aria-label="Previous cascade step"
          onClick={() => prevCascade()}
        >
          ◀
        </button>
        <span className="text-[10px] text-gray-500 font-mono min-w-[16px] text-center">
          {cascadeIndex}
        </span>
        <button
          className="px-1.5 py-0.5 text-[10px] rounded bg-gray-800 hover:bg-gray-700 text-gray-400 transition-colors"
          aria-label="Next cascade step"
          onClick={() => nextCascade()}
        >
          ▶
        </button>
      </div>
    </div>
  );
}
