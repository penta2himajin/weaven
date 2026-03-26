import { useCallback, useEffect, useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  BackgroundVariant,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import dagre from "dagre";
import SmNode from "./SmNode";
import { applyHighlights } from "./topologyHelpers";
import { useDebugStore } from "../stores/debugStore";

const NODE_WIDTH = 180;
const NODE_HEIGHT = 80;

const nodeTypes = { smNode: SmNode };

/** Extract raw number from newtype ({ inner: N }) or plain number. */
function unwrapId(v: unknown): number {
  if (typeof v === "number") return v;
  if (v && typeof (v as any).inner === "number") return (v as any).inner;
  return 0;
}

/** Convert topology graph data → React Flow nodes + edges with dagre layout. */
function layoutGraph(
  topoNodes: { sm_id: unknown; active_state: unknown; label: string }[],
  topoEdges: { source: unknown; target: unknown; kind: string; label: string }[],
): { nodes: Node[]; edges: Edge[] } {
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: "LR", nodesep: 40, ranksep: 80 });

  const rfNodes: Node[] = topoNodes.map((n) => {
    const smId = unwrapId(n.sm_id);
    const id = `sm-${smId}`;
    g.setNode(id, { width: NODE_WIDTH, height: NODE_HEIGHT });
    return {
      id,
      type: "smNode",
      position: { x: 0, y: 0 },
      data: {
        label: n.label,
        smId,
        activeState: n.active_state != null ? unwrapId(n.active_state) : null,
      },
    };
  });

  const rfEdges: Edge[] = topoEdges.map((e, i) => {
    const sourceId = `sm-${unwrapId(e.source)}`;
    const targetId = `sm-${unwrapId(e.target)}`;
    g.setEdge(sourceId, targetId);

    const style = edgeStyle(e.kind);

    return {
      id: `edge-${i}`,
      source: sourceId,
      target: targetId,
      label: e.label,
      style,
      animated: e.kind === "IR",
    };
  });

  dagre.layout(g);

  for (const node of rfNodes) {
    const pos = g.node(node.id);
    if (pos) {
      node.position = {
        x: pos.x - NODE_WIDTH / 2,
        y: pos.y - NODE_HEIGHT / 2,
      };
    }
  }

  return { nodes: rfNodes, edges: rfEdges };
}

function edgeStyle(kind: string): React.CSSProperties {
  switch (kind) {
    case "Static":
      return { stroke: "#6366f1", strokeWidth: 2 };
    case "Spatial":
      return { stroke: "#22d3ee", strokeWidth: 2, strokeDasharray: "6 3" };
    case "IR":
      return { stroke: "#f59e0b", strokeWidth: 1.5, strokeDasharray: "3 3" };
    default:
      return { stroke: "#9ca3af" };
  }
}

export default function TopologyCanvas() {
  const topology = useDebugStore((s) => s.topology);
  const selectSm = useDebugStore((s) => s.selectSm);
  const highlightedEdges = useDebugStore((s) => s.highlightedEdges);
  // Subscribe to dependencies of highlightedEdges() to trigger re-render.
  useDebugStore((s) => s.selectedTraceIndex);
  useDebugStore((s) => s.traceEvents);
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  useEffect(() => {
    if (!topology) return;
    const { nodes: n, edges: e } = layoutGraph(topology.nodes, topology.edges);
    setNodes(n);
    setEdges(e);
  }, [topology, setNodes, setEdges]);

  // Apply highlight styles to edges when selection changes.
  const highlights = highlightedEdges();
  const styledEdges = useMemo(
    () => applyHighlights(edges as any, highlights) as any,
    [edges, highlights],
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      const smId = node.data?.smId as number | undefined;
      if (smId != null) {
        selectSm({ inner: smId } as any);
      }
    },
    [selectSm],
  );

  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={styledEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClick}
        nodeTypes={nodeTypes}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background variant={BackgroundVariant.Dots} gap={16} size={1} color="#1f2937" />
        <Controls className="!bg-gray-900 !border-gray-700" />
      </ReactFlow>
    </div>
  );
}
