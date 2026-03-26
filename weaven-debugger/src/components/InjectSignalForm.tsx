import { useState, useCallback } from "react";
import { useDebugStore } from "../stores/debugStore";
import { useCommands } from "./CommandsContext";

/**
 * Compact form for injecting a debug signal into a selected SM's port.
 * Only visible when an SM is selected.
 */
export default function InjectSignalForm() {
  const selectedSmId = useDebugStore((s) => s.selectedSmId);

  let cmds: ReturnType<typeof useCommands> | null = null;
  try {
    cmds = useCommands();
  } catch {
    // No provider.
  }

  const smIdNum = selectedSmId != null
    ? (typeof selectedSmId === "number" ? selectedSmId : (selectedSmId as any).inner ?? null)
    : null;

  const [portId, setPortId] = useState("0");
  const [payloadStr, setPayloadStr] = useState("{}");
  const [error, setError] = useState<string | null>(null);

  const handleInject = useCallback(async () => {
    if (smIdNum == null || !cmds) return;
    try {
      const parsed = JSON.parse(payloadStr);
      setError(null);
      await cmds.injectSignal(smIdNum, parseInt(portId, 10), parsed);
    } catch (e: any) {
      setError(e.message ?? "Invalid payload JSON");
    }
  }, [smIdNum, portId, payloadStr, cmds]);

  if (smIdNum == null) return null;

  return (
    <div className="border-t border-gray-800 px-3 py-2 space-y-2">
      <h3 className="text-[10px] font-semibold text-gray-500 uppercase">
        Inject Signal
      </h3>
      <div className="flex items-center gap-2">
        <label className="text-[10px] text-gray-500">
          SM
          <input
            type="number"
            value={smIdNum}
            readOnly
            className="ml-1 w-12 bg-gray-800 text-gray-300 text-xs px-1 py-0.5 rounded border border-gray-700"
            aria-label="SM"
          />
        </label>
        <label className="text-[10px] text-gray-500">
          Port
          <input
            type="number"
            value={portId}
            onChange={(e) => setPortId(e.target.value)}
            className="ml-1 w-12 bg-gray-800 text-gray-300 text-xs px-1 py-0.5 rounded border border-gray-700"
            aria-label="Port"
          />
        </label>
      </div>
      <label className="block text-[10px] text-gray-500">
        Payload
        <input
          type="text"
          value={payloadStr}
          onChange={(e) => setPayloadStr(e.target.value)}
          className="mt-0.5 w-full bg-gray-800 text-gray-300 text-xs px-1.5 py-0.5 rounded border border-gray-700 font-mono"
          placeholder='{"intensity": 5.0}'
          aria-label="Payload"
        />
      </label>
      {error && <p className="text-[10px] text-red-400">{error}</p>}
      <button
        onClick={handleInject}
        className="px-2 py-0.5 text-[10px] rounded bg-amber-700 hover:bg-amber-600 text-white transition-colors"
        aria-label="Inject"
      >
        Inject
      </button>
    </div>
  );
}
