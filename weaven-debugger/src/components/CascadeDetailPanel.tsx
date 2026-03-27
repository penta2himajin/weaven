import { useDebugStore } from "../stores/debugStore";

export default function CascadeDetailPanel() {
  const cascadeIndex = useDebugStore((s) => s.cascadeIndex);
  const cascadeSteps = useDebugStore((s) => s.cascadeSteps);
  const signalsForCascadeStep = useDebugStore((s) => s.signalsForCascadeStep);
  // Subscribe to traceEvents to trigger re-render.
  useDebugStore((s) => s.traceEvents);

  const steps = cascadeSteps();
  if (steps.length === 0) {
    return (
      <div className="px-3 py-2">
        <p className="text-xs text-gray-600">No cascade this tick</p>
      </div>
    );
  }

  const currentStep: any = steps[cascadeIndex];
  const signals = signalsForCascadeStep(cascadeIndex);

  return (
    <div className="px-3 py-2 space-y-2" data-testid="cascade-detail-panel">
      <div className="flex items-center gap-2">
        <span className="text-[10px] font-semibold text-gray-500 uppercase">
          Cascade Depth
        </span>
        <span className="text-xs font-mono text-amber-400">
          {currentStep?.depth ?? 0}
        </span>
        <span className="text-[10px] text-gray-600">
          ({currentStep?.queueSize ?? 0} signals)
        </span>
      </div>

      {signals.length === 0 ? (
        <p className="text-xs text-gray-600">No signal deliveries at this depth</p>
      ) : (
        <div className="space-y-1">
          {signals.map((sig: any, i: number) => {
            const sourceSm = sig.sourceSm?.inner ?? sig.sourceSm ?? "?";
            const targetSm = sig.targetSm?.inner ?? sig.targetSm ?? "?";
            const targetPort = sig.targetPort?.inner ?? sig.targetPort ?? "?";
            const tid = sig.triggeredTransition?.inner ?? sig.triggeredTransition;
            return (
              <div
                key={i}
                className="flex items-center gap-2 text-[10px] font-mono"
                data-testid="signal-delivery"
              >
                <span className="text-cyan-400">
                  SM({String(sourceSm)})
                </span>
                <span className="text-gray-600">{"\u2192"}</span>
                <span className="text-indigo-400">
                  SM({String(targetSm)}):{String(targetPort)}
                </span>
                {tid != null ? (
                  <span className="text-emerald-400">T({String(tid)})</span>
                ) : (
                  <span className="text-gray-600">no transition</span>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
