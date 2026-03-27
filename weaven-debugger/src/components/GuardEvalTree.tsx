import type { EvalTreeNode } from "../generated/models";

interface Props {
  tree: EvalTreeNode;
}

function nodeColor(value: number): string {
  return value !== 0 ? "text-emerald-400" : "text-red-400";
}

function EvalNode({ node, depth }: { node: EvalTreeNode; depth: number }) {
  const indent = depth * 12;
  const hasChildren = node.children && node.children.length > 0;

  return (
    <div style={{ marginLeft: indent }}>
      <div className="flex items-center gap-1.5 py-0.5">
        <span className={`text-[10px] font-mono ${nodeColor(node.value)}`}>
          {node.value !== 0 ? "\u2714" : "\u2718"}
        </span>
        <span className="text-[10px] font-mono text-gray-500">
          {node.exprKind}
        </span>
        <span className="text-[10px] font-mono text-gray-300">
          {node.label}
        </span>
        <span className={`text-[10px] font-mono ${nodeColor(node.value)}`}>
          = {formatValue(node.value)}
        </span>
      </div>
      {hasChildren &&
        node.children.map((child, i) => (
          <EvalNode key={i} node={child} depth={depth + 1} />
        ))}
    </div>
  );
}

function formatValue(v: number): string {
  if (v === 0 || v === 1) return v === 1 ? "true" : "false";
  if (Number.isInteger(v)) return String(v);
  return v.toFixed(2);
}

export default function GuardEvalTree({ tree }: Props) {
  return (
    <div className="border border-gray-700 rounded p-1.5 bg-gray-900/50" data-testid="guard-eval-tree">
      <EvalNode node={tree} depth={0} />
    </div>
  );
}
