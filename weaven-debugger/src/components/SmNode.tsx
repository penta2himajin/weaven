import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";

interface SmNodeData {
  label: string;
  smId: number;
  activeState: number | null;
  diffChanged?: boolean;
  [key: string]: unknown;
}

/** SM node — shows SM name, active state, and input/output handles. */
function SmNodeComponent({ data, selected }: NodeProps) {
  const { label, smId, activeState, diffChanged } = data as SmNodeData;

  const borderClass = diffChanged
    ? "border-amber-400 ring-1 ring-amber-400/50"
    : selected
      ? "border-indigo-400 ring-1 ring-indigo-400/50"
      : "border-gray-700";

  return (
    <div
      className={`
        rounded-lg border px-3 py-2 min-w-[160px]
        bg-gray-900 shadow-lg
        ${borderClass}
      `}
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-1">
        <span className="text-xs font-semibold text-gray-300">{label}</span>
        <span className="text-[10px] text-gray-600">#{smId}</span>
      </div>

      {/* Active state */}
      <div className="flex items-center gap-1.5">
        <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
        <span className="text-xs text-emerald-300 font-mono">
          {activeState != null ? `S(${activeState})` : "—"}
        </span>
      </div>

      {/* Handles */}
      <Handle
        type="target"
        position={Position.Left}
        className="!bg-indigo-500 !w-2.5 !h-2.5 !border-gray-900"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!bg-amber-500 !w-2.5 !h-2.5 !border-gray-900"
      />
    </div>
  );
}

export default memo(SmNodeComponent);
