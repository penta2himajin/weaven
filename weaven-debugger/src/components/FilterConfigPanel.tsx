import { useDebugStore } from "../stores/debugStore";

const PHASES = [
  { key: "Evaluate", label: "Eval" },
  { key: "Execute", label: "Exec" },
  { key: "Propagate", label: "Prop" },
] as const;

export default function FilterConfigPanel() {
  const topology = useDebugStore((s) => s.topology);
  const filterConfig = useDebugStore((s) => s.filterConfig);
  const toggleSm = useDebugStore((s) => s.toggleSmVisibility);
  const togglePhase = useDebugStore((s) => s.togglePhaseVisibility);
  const resetFilter = useDebugStore((s) => s.resetFilter);

  const nodes = topology?.nodes ?? [];

  return (
    <div className="px-3 py-2 space-y-2 border-t border-gray-800">
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider">
          Filter
        </span>
        <button
          onClick={resetFilter}
          aria-label="Reset filters"
          className="text-[10px] text-gray-600 hover:text-gray-400 transition-colors"
        >
          Reset
        </button>
      </div>

      {/* Phase toggles */}
      <div className="flex gap-1 flex-wrap">
        {PHASES.map(({ key, label }) => {
          const hidden = filterConfig.hiddenPhases.has(key);
          return (
            <button
              key={key}
              onClick={() => togglePhase(key)}
              className={`px-1.5 py-0.5 text-[10px] rounded transition-colors ${
                hidden
                  ? "bg-gray-800 text-gray-600 line-through"
                  : "bg-gray-700 text-gray-300"
              }`}
            >
              {label}
            </button>
          );
        })}
      </div>

      {/* SM toggles */}
      {nodes.length > 0 && (
        <div className="flex gap-1 flex-wrap">
          {nodes.map((n) => {
            const id = typeof n.sm_id === "number" ? n.sm_id : (n.sm_id as any).inner;
            const hidden = filterConfig.hiddenSmIds.has(id);
            return (
              <button
                key={id}
                onClick={() => toggleSm(id)}
                className={`px-1.5 py-0.5 text-[10px] rounded font-mono transition-colors ${
                  hidden
                    ? "bg-gray-800 text-gray-600 line-through"
                    : "bg-gray-700 text-gray-300"
                }`}
              >
                {n.label}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
