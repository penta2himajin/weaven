import { useDebugStore } from "../stores/debugStore";
import InjectSignalForm from "./InjectSignalForm";
import GuardEvalTree from "./GuardEvalTree";

export default function InspectorPanel() {
  const selectedSmId = useDebugStore((s) => s.selectedSmId);
  const filteredTraceEvents = useDebugStore((s) => s.filteredTraceEvents);
  const diffForSm = useDebugStore((s) => s.diffForSm);
  // Subscribe to traceEvents and diffs to trigger re-render.
  useDebugStore((s) => s.traceEvents);
  useDebugStore((s) => s.diffs);

  if (!selectedSmId) {
    return (
      <div className="flex flex-col h-full">
        <div className="px-3 py-2 border-b border-gray-800">
          <span className="text-xs font-semibold text-gray-400 uppercase tracking-wider">
            Inspector
          </span>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="text-xs text-gray-600">Select an SM node to inspect</p>
        </div>
      </div>
    );
  }

  const filtered = filteredTraceEvents();
  const guardEvents = filtered.filter((e: any) => e.kind === "GuardEvaluated");
  const firedEvents = filtered.filter((e: any) => e.kind === "TransitionFired");

  const smIdDisplay = typeof selectedSmId === "object" && "inner" in selectedSmId
    ? (selectedSmId as any).inner
    : selectedSmId;

  const diff = diffForSm(smIdDisplay as number);

  return (
    <div className="flex flex-col h-full">
      <div className="px-3 py-2 border-b border-gray-800 flex items-center justify-between">
        <span className="text-xs font-semibold text-gray-400 uppercase tracking-wider">
          Inspector
        </span>
        <span className="text-xs text-indigo-400 font-mono">
          SM({smIdDisplay})
        </span>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-2 space-y-3">
        {/* Guard evaluations */}
        <section>
          <h3 className="text-[10px] font-semibold text-gray-500 uppercase mb-1">
            Guard Evaluations
          </h3>
          {guardEvents.length === 0 ? (
            <p className="text-xs text-gray-600">None this tick</p>
          ) : (
            <div className="space-y-1">
              {guardEvents.map((e: any, i: number) => {
                const tid = e.transition?.inner ?? e.transition;
                const snapshot: [string, number][] | null = e.contextSnapshot ?? null;
                const evalTree = e.evalTree ?? null;
                return (
                  <div key={i} className="space-y-0.5">
                    <div className="flex items-center gap-2 text-xs font-mono">
                      <span className={e.result ? "text-emerald-400" : "text-red-400"}>
                        {e.result ? "\u2713" : "\u2717"}
                      </span>
                      <span className="text-gray-400">T({String(tid)})</span>
                    </div>
                    {evalTree && (
                      <GuardEvalTree tree={evalTree} />
                    )}
                    {!evalTree && !e.result && snapshot && snapshot.length > 0 && (
                      <div className="ml-4 pl-2 border-l border-gray-700 space-y-0.5">
                        {snapshot.map(([k, v]) => (
                          <div key={k} className="flex gap-2 text-[10px] font-mono text-gray-500">
                            <span className="text-gray-600">{k}</span>
                            <span className="text-amber-400">{String(v)}</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </section>

        {/* Transitions fired */}
        <section>
          <h3 className="text-[10px] font-semibold text-gray-500 uppercase mb-1">
            Transitions Fired
          </h3>
          {firedEvents.length === 0 ? (
            <p className="text-xs text-gray-600">None this tick</p>
          ) : (
            <div className="space-y-1">
              {firedEvents.map((e: any, i: number) => {
                const from = e.fromState?.inner ?? e.fromState;
                const to = e.toState?.inner ?? e.toState;
                return (
                  <div key={i} className="text-xs font-mono text-gray-300">
                    S({String(from)}) {"\u2192"} S({String(to)})
                  </div>
                );
              })}
            </div>
          )}
        </section>

        {/* Network sync diff */}
        {diff && (
          <section data-testid="diff-section">
            <h3 className="text-[10px] font-semibold text-gray-500 uppercase mb-1">
              Network Diff
            </h3>
            <div className="space-y-1">
              {diff.prevState !== diff.newState && (
                <div className="text-[10px] font-mono">
                  <span className="text-gray-500">state: </span>
                  <span className="text-red-400">S({diff.prevState})</span>
                  <span className="text-gray-600"> {"\u2192"} </span>
                  <span className="text-emerald-400">S({diff.newState})</span>
                </div>
              )}
              {Object.entries(diff.contextChanges).map(([k, v]) => (
                <div key={k} className="flex gap-2 text-[10px] font-mono" data-testid="diff-field">
                  <span className="text-gray-500">{k}:</span>
                  <span className="text-amber-400">{String(v)}</span>
                </div>
              ))}
            </div>
          </section>
        )}
      </div>

      {/* Inject signal form */}
      <InjectSignalForm />
    </div>
  );
}
