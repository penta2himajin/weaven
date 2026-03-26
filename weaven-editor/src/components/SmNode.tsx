import { Handle, Position } from "@xyflow/react";
import type { SmNodeData } from "./topologyHelpers";

export default function SmNode({ data }: { data: SmNodeData }) {
  return (
    <div className="bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 min-w-[160px]">
      <div className="font-semibold text-sm text-gray-100 mb-1">
        {data.label}
      </div>
      <div className="text-xs text-gray-400">
        States: [{data.states.join(", ")}]
      </div>
      <div className="text-xs text-gray-400">
        Initial: {data.initialState}
      </div>
      {data.inputPorts.map((port, i) => (
        <Handle
          key={`in-${port.id}`}
          type="target"
          position={Position.Left}
          id={`in-${port.id}`}
          className="!bg-indigo-400"
          style={{ top: `${30 + i * 20}%` }}
        />
      ))}
      {data.outputPorts.map((port, i) => (
        <Handle
          key={`out-${port.id}`}
          type="source"
          position={Position.Right}
          id={`out-${port.id}`}
          className="!bg-emerald-400"
          style={{ top: `${30 + i * 20}%` }}
        />
      ))}
    </div>
  );
}
