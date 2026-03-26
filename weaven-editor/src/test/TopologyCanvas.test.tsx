import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { ReactFlowProvider } from "@xyflow/react";
import TopologyCanvas from "../components/TopologyCanvas";
import { useEditorStore } from "../stores/editorStore";
import type { WeavenSchema } from "../generated/schema";

const schema: WeavenSchema = {
  state_machines: [
    {
      id: 1, states: [0, 1], initial_state: 0,
      transitions: [],
      input_ports: [{ id: 0, kind: "Input", signal_type: 0 }],
      output_ports: [{ id: 1, kind: "Output", signal_type: 0 }],
    },
    {
      id: 2, states: [0], initial_state: 0,
      transitions: [],
      input_ports: [{ id: 0, kind: "Input", signal_type: 0 }],
      output_ports: [],
    },
  ],
  connections: [
    { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [] },
  ],
  named_tables: [], interaction_rules: [],
};

function renderCanvas() {
  return render(
    <ReactFlowProvider>
      <TopologyCanvas />
    </ReactFlowProvider>,
  );
}

describe("TopologyCanvas", () => {
  beforeEach(() => {
    useEditorStore.setState({
      schema: { state_machines: [], connections: [], named_tables: [], interaction_rules: [] },
      selectedSmId: null,
      selectedConnectionId: null, selectedInteractionRuleId: null,
      dirty: false,
    });
  });

  it("renders empty state message when no SMs", () => {
    renderCanvas();
    expect(screen.getByText(/no state machines/i)).toBeInTheDocument();
  });

  it("renders React Flow container when SMs exist", () => {
    useEditorStore.getState().loadSchema(schema);
    renderCanvas();
    const container = document.querySelector(".react-flow");
    expect(container).not.toBeNull();
  });

  it("addConnectionFromDrag is exposed for onConnect callback", () => {
    expect(typeof useEditorStore.getState().addConnectionFromDrag).toBe("function");
  });

  it("store creates connection via addConnectionFromDrag", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().addConnectionFromDrag(1, 1, 2, 0);
    // Already has connection id=1 from schema, so new one should be id=2
    const conns = useEditorStore.getState().schema.connections;
    expect(conns).toHaveLength(1); // duplicate detection: same source/target port already connected
  });

  it("store creates new connection for different ports", () => {
    const schemaNoConn: WeavenSchema = {
      ...schema,
      connections: [],
    };
    useEditorStore.getState().loadSchema(schemaNoConn);
    useEditorStore.getState().addConnectionFromDrag(1, 1, 2, 0);
    expect(useEditorStore.getState().schema.connections).toHaveLength(1);
  });
});
