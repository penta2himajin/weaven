import dagre from "dagre";
import type { Node, Edge } from "@xyflow/react";
import type { WeavenSchema, PortSchema } from "../generated/schema";

export interface SmNodeData {
  smId: number;
  label: string;
  states: number[];
  initialState: number;
  inputPorts: PortSchema[];
  outputPorts: PortSchema[];
  [key: string]: unknown;
}

const NODE_WIDTH = 180;
const NODE_HEIGHT = 80;

export function schemaToNodes(schema: WeavenSchema): Node<SmNodeData>[] {
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: "LR", nodesep: 60, ranksep: 120 });

  for (const sm of schema.state_machines) {
    g.setNode(`sm-${sm.id}`, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }
  for (const c of schema.connections) {
    g.setEdge(`sm-${c.source_sm}`, `sm-${c.target_sm}`);
  }

  dagre.layout(g);

  return schema.state_machines.map((sm) => {
    const pos = g.node(`sm-${sm.id}`);
    return {
      id: `sm-${sm.id}`,
      type: "smNode",
      position: {
        x: pos ? pos.x - NODE_WIDTH / 2 : 0,
        y: pos ? pos.y - NODE_HEIGHT / 2 : 0,
      },
      data: {
        smId: sm.id,
        label: `SM(${sm.id})`,
        states: sm.states,
        initialState: sm.initial_state,
        inputPorts: sm.input_ports,
        outputPorts: sm.output_ports,
      },
    };
  });
}

export function schemaToEdges(schema: WeavenSchema): Edge[] {
  return schema.connections.map((c) => ({
    id: `conn-${c.id}`,
    source: `sm-${c.source_sm}`,
    target: `sm-${c.target_sm}`,
    data: {
      connectionId: c.id,
      delayTicks: c.delay_ticks,
      pipelineSteps: c.pipeline.length,
    },
  }));
}
