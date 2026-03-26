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
  named_tables: [],
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
      schema: { state_machines: [], connections: [], named_tables: [] },
      selectedSmId: null,
      selectedConnectionId: null,
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
    // React Flow renders a container with role="application" or the class
    const container = document.querySelector(".react-flow");
    expect(container).not.toBeNull();
  });
});
