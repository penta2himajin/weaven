import { useMemo, useCallback } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  type NodeMouseHandler,
  type EdgeMouseHandler,
  type OnConnect,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useEditorStore } from "../stores/editorStore";
import { schemaToNodes, schemaToEdges } from "./topologyHelpers";
import SmNode from "./SmNode";

const nodeTypes = { smNode: SmNode };

export default function TopologyCanvas() {
  const schema = useEditorStore((s) => s.schema);
  const selectSm = useEditorStore((s) => s.selectSm);
  const selectConnection = useEditorStore((s) => s.selectConnection);
  const addConnectionFromDrag = useEditorStore((s) => s.addConnectionFromDrag);

  const nodes = useMemo(() => schemaToNodes(schema), [schema]);
  const edges = useMemo(() => schemaToEdges(schema), [schema]);

  const onNodeClick: NodeMouseHandler = useCallback(
    (_event, node) => {
      const smId = parseInt(node.id.replace("sm-", ""), 10);
      selectSm(smId);
    },
    [selectSm],
  );

  const onEdgeClick: EdgeMouseHandler = useCallback(
    (_event, edge) => {
      const connId = edge.data?.connectionId as number | undefined;
      if (connId != null) {
        selectConnection(connId);
      }
    },
    [selectConnection],
  );

  const onConnect: OnConnect = useCallback(
    (params) => {
      const sourceSm = parseInt(params.source.replace("sm-", ""), 10);
      const targetSm = parseInt(params.target.replace("sm-", ""), 10);
      const sourcePort = parseInt((params.sourceHandle ?? "").replace("out-", ""), 10);
      const targetPort = parseInt((params.targetHandle ?? "").replace("in-", ""), 10);
      if (!isNaN(sourceSm) && !isNaN(targetSm) && !isNaN(sourcePort) && !isNaN(targetPort)) {
        addConnectionFromDrag(sourceSm, sourcePort, targetSm, targetPort);
      }
    },
    [addConnectionFromDrag],
  );

  if (schema.state_machines.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No state machines defined. Click &quot;Add SM&quot; to start.
      </div>
    );
  }

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodeClick={onNodeClick}
      onEdgeClick={onEdgeClick}
      onConnect={onConnect}
      fitView
    >
      <Background />
      <Controls />
    </ReactFlow>
  );
}
