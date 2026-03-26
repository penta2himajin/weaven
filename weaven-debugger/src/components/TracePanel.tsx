import { useDebugStore } from "../stores/debugStore";
import type { TraceEvent } from "../generated/models";

function phaseBadge(phase: string): string {
  switch (phase) {
    case "Evaluate":  return "bg-blue-900 text-blue-300";
    case "Execute":   return "bg-purple-900 text-purple-300";
    case "Propagate": return "bg-amber-900 text-amber-300";
    default:          return "bg-gray-800 text-gray-400";
  }
}

function TraceRow({ event, index, selected, onSelect }: { event: TraceEvent; index: number; selected: boolean; onSelect: (i: number) => void }) {
  const kind = event.kind;
  const phase = "phase" in event ? String(event.phase) : "";

  let detail = "";
  switch (event.kind) {
    case "GuardEvaluated":
      detail = `T(${(event as any).transition?.inner ?? event.transition}) SM(${(event as any).smId?.inner ?? event.smId}) → ${event.result ? "✓" : "✗"}`;
      break;
    case "TransitionFired":
      detail = `SM(${(event as any).smId?.inner ?? event.smId}) S(${(event as any).fromState?.inner ?? event.fromState})→S(${(event as any).toState?.inner ?? event.toState})`;
      break;
    case "SignalEmitted":
      detail = `SM(${(event as any).smId?.inner ?? event.smId}) P(${(event as any).port?.inner ?? event.port})`;
      break;
    case "CascadeStep":
      detail = `depth=${event.depth} queue=${event.queueSize}`;
      break;
    case "PipelineFiltered":
      detail = `SM(${(event as any).smId?.inner ?? event.smId}) P(${(event as any).port?.inner ?? event.port}) blocked`;
      break;
    case "IrMatched":
      detail = `IR(${(event as any).ruleId?.inner ?? event.ruleId}) [${event.participants?.size ?? 0} SMs]`;
      break;
  }

  return (
    <div
      className={`flex items-center gap-2 px-2 py-0.5 text-xs hover:bg-gray-800/50 cursor-pointer font-mono ${selected ? "bg-indigo-900/30 border-l-2 border-indigo-400" : ""}`}
      data-trace-index={index}
      onClick={() => onSelect(index)}
    >
      <span className={`px-1.5 py-0.5 rounded text-[10px] ${phaseBadge(phase)}`}>
        {String(phase).slice(0, 4)}
      </span>
      <span className="text-gray-500 w-24 truncate">{kind}</span>
      <span className="text-gray-300 flex-1 truncate">{detail}</span>
    </div>
  );
}

export default function TracePanel() {
  const filteredTraceEvents = useDebugStore((s) => s.filteredTraceEvents);
  const traceEvents = useDebugStore((s) => s.traceEvents);
  const selectedTraceIndex = useDebugStore((s) => s.selectedTraceIndex);
  const selectTraceEvent = useDebugStore((s) => s.selectTraceEvent);
  // Subscribe to filter inputs so we re-render when they change.
  useDebugStore((s) => s.selectedSmId);
  useDebugStore((s) => s.filterConfig);
  const filtered = filteredTraceEvents();

  // Map filtered events back to their original index in traceEvents.
  const withIndices = filtered.map((event) => {
    const origIdx = traceEvents.indexOf(event);
    return { event, origIdx };
  });

  return (
    <div className="flex flex-col h-full">
      <div className="px-3 py-2 border-b border-gray-800 flex items-center justify-between">
        <span className="text-xs font-semibold text-gray-400 uppercase tracking-wider">
          Trace
        </span>
        <span className="text-[10px] text-gray-600">{filtered.length} events</span>
      </div>
      <div className="flex-1 overflow-y-auto">
        {filtered.length === 0 ? (
          <div className="px-3 py-4 text-xs text-gray-600 text-center">
            No trace events
          </div>
        ) : (
          withIndices.map(({ event, origIdx }) => (
            <TraceRow
              key={origIdx}
              event={event}
              index={origIdx}
              selected={selectedTraceIndex === origIdx}
              onSelect={selectTraceEvent}
            />
          ))
        )}
      </div>
    </div>
  );
}
