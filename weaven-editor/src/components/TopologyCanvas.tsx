import { useMemo, useCallback } from "react";
import { ReactFlow, Background, Controls, type NodeMouseHandler } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useEditorStore } from "../stores/editorStore";
import { schemaToNodes, schemaToEdges } from "./topologyHelpers";
import SmNode from "./SmNode";

const nodeTypes = { smNode: SmNode };

export default function TopologyCanvas() {
  const schema = useEditorStore((s) => s.schema);
  const selectSm = useEditorStore((s) => s.selectSm);

  const nodes = useMemo(() => schemaToNodes(schema), [schema]);
  const edges = useMemo(() => schemaToEdges(schema), [schema]);

  const onNodeClick: NodeMouseHandler = useCallback(
    (_event, node) => {
      const smId = parseInt(node.id.replace("sm-", ""), 10);
      selectSm(smId);
    },
    [selectSm],
  );

  if (schema.state_machines.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No state machines defined. Click "Add SM" to start.
      </div>
    );
  }

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodeClick={onNodeClick}
      fitView
    >
      <Background />
      <Controls />
    </ReactFlow>
  );
}
