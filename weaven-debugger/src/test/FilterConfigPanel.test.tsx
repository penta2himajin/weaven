import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import FilterConfigPanel from "../components/FilterConfigPanel";
import { useDebugStore } from "../stores/debugStore";

beforeEach(() => {
  useDebugStore.setState({
    loaded: true,
    currentTick: 1,
    maxTick: 1,
    topology: {
      nodes: [
        { sm_id: { inner: 1 }, active_state: { inner: 0 }, label: "SM(1)" },
        { sm_id: { inner: 2 }, active_state: { inner: 0 }, label: "SM(2)" },
        { sm_id: { inner: 3 }, active_state: null, label: "SM(3)" },
      ],
      edges: [],
    },
    traceEvents: [],
    selectedSmId: null,
    cascadeIndex: 0,
    selectedTraceIndex: null,
    filterConfig: { hiddenSmIds: new Set(), hiddenPhases: new Set() },
  });
});

describe("FilterConfigPanel", () => {
  it("renders phase toggles", () => {
    render(<FilterConfigPanel />);
    expect(screen.getByText("Eval")).toBeInTheDocument();
    expect(screen.getByText("Exec")).toBeInTheDocument();
    expect(screen.getByText("Prop")).toBeInTheDocument();
  });

  it("renders SM toggles from topology", () => {
    render(<FilterConfigPanel />);
    expect(screen.getByText("SM(1)")).toBeInTheDocument();
    expect(screen.getByText("SM(2)")).toBeInTheDocument();
    expect(screen.getByText("SM(3)")).toBeInTheDocument();
  });

  it("clicking a phase toggle hides that phase", () => {
    render(<FilterConfigPanel />);
    fireEvent.click(screen.getByText("Eval"));
    expect(useDebugStore.getState().filterConfig.hiddenPhases.has("Evaluate")).toBe(true);
  });

  it("clicking an SM toggle hides that SM", () => {
    render(<FilterConfigPanel />);
    fireEvent.click(screen.getByText("SM(2)"));
    expect(useDebugStore.getState().filterConfig.hiddenSmIds.has(2)).toBe(true);
  });

  it("reset button clears all filters", () => {
    useDebugStore.getState().toggleSmVisibility(1);
    useDebugStore.getState().togglePhaseVisibility("Execute");

    render(<FilterConfigPanel />);
    fireEvent.click(screen.getByRole("button", { name: /reset/i }));

    const { filterConfig } = useDebugStore.getState();
    expect(filterConfig.hiddenSmIds.size).toBe(0);
    expect(filterConfig.hiddenPhases.size).toBe(0);
  });
});
