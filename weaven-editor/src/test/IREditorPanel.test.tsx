import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import IREditorPanel from "../components/IREditorPanel";
import { useEditorStore } from "../stores/editorStore";

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [], interaction_rules: [] },
    selectedSmId: null,
    selectedConnectionId: null,
    selectedInteractionRuleId: null,
    dirty: false,
  });
}

describe("IREditorPanel", () => {
  beforeEach(resetStore);

  it("shows empty message when no IRs exist", () => {
    render(<IREditorPanel />);
    expect(screen.getByText(/no interaction rules/i)).toBeInTheDocument();
  });

  it("Add IR button creates a new interaction rule", async () => {
    const user = userEvent.setup();
    render(<IREditorPanel />);
    await user.click(screen.getByRole("button", { name: /add ir/i }));
    expect(useEditorStore.getState().schema.interaction_rules).toHaveLength(1);
  });

  it("displays IR list after adding", async () => {
    const user = userEvent.setup();
    render(<IREditorPanel />);
    await user.click(screen.getByRole("button", { name: /add ir/i }));
    expect(screen.getByText(/IR\(1\)/)).toBeInTheDocument();
  });

  it("clicking an IR selects it and shows detail panel", async () => {
    const user = userEvent.setup();
    render(<IREditorPanel />);
    await user.click(screen.getByRole("button", { name: /add ir/i }));
    await user.click(screen.getByText(/IR\(1\)/));
    expect(useEditorStore.getState().selectedInteractionRuleId).toBe(1);
    expect(screen.getByText("Participants")).toBeInTheDocument();
    expect(screen.getByText("Conditions")).toBeInTheDocument();
  });

  it("Delete IR button removes the rule", async () => {
    const user = userEvent.setup();
    render(<IREditorPanel />);
    await user.click(screen.getByRole("button", { name: /add ir/i }));
    await user.click(screen.getByText(/IR\(1\)/));
    await user.click(screen.getByRole("button", { name: /delete ir/i }));
    expect(useEditorStore.getState().schema.interaction_rules).toHaveLength(0);
  });

  it("Add Participant adds a participant to the selected IR", async () => {
    const user = userEvent.setup();
    render(<IREditorPanel />);
    await user.click(screen.getByRole("button", { name: /add ir/i }));
    await user.click(screen.getByText(/IR\(1\)/));

    const smIdInput = screen.getByLabelText("participant SM ID");
    await user.type(smIdInput, "1");
    await user.click(screen.getByRole("button", { name: /add participant/i }));

    const rule = useEditorStore.getState().schema.interaction_rules[0];
    expect(rule.participants).toHaveLength(1);
    expect(rule.participants[0].sm_id).toBe(1);
  });

  it("Remove Participant removes a participant", async () => {
    useEditorStore.getState().addInteractionRule();
    useEditorStore.getState().selectInteractionRule(1);
    useEditorStore.getState().updateInteractionRule(1, {
      participants: [{ sm_id: 1 }, { sm_id: 2 }],
    });

    const user = userEvent.setup();
    render(<IREditorPanel />);

    const removeButtons = screen.getAllByLabelText(/remove participant/i);
    await user.click(removeButtons[0]);

    const rule = useEditorStore.getState().schema.interaction_rules[0];
    expect(rule.participants).toHaveLength(1);
    expect(rule.participants[0].sm_id).toBe(2);
  });

  it("Add Condition adds a Spatial condition", async () => {
    const user = userEvent.setup();
    render(<IREditorPanel />);
    await user.click(screen.getByRole("button", { name: /add ir/i }));
    await user.click(screen.getByText(/IR\(1\)/));

    await user.click(screen.getByRole("button", { name: /add condition/i }));

    const rule = useEditorStore.getState().schema.interaction_rules[0];
    expect(rule.conditions).toHaveLength(1);
    expect(rule.conditions[0].kind).toBe("Spatial");
  });

  it("Remove Condition removes a condition", async () => {
    useEditorStore.getState().addInteractionRule();
    useEditorStore.getState().selectInteractionRule(1);
    useEditorStore.getState().updateInteractionRule(1, {
      conditions: [{ kind: "Spatial", radius: 10 }],
    });

    const user = userEvent.setup();
    render(<IREditorPanel />);

    await user.click(screen.getByLabelText(/remove condition 0/i));

    const rule = useEditorStore.getState().schema.interaction_rules[0];
    expect(rule.conditions).toHaveLength(0);
  });

  it("shows condition details", async () => {
    useEditorStore.getState().addInteractionRule();
    useEditorStore.getState().selectInteractionRule(1);
    useEditorStore.getState().updateInteractionRule(1, {
      conditions: [{ kind: "Spatial", radius: 15 }],
    });

    render(<IREditorPanel />);
    expect(screen.getByText(/Spatial \(radius: 15\)/)).toBeInTheDocument();
  });

  it("shows participant with required state", async () => {
    useEditorStore.getState().addInteractionRule();
    useEditorStore.getState().selectInteractionRule(1);
    useEditorStore.getState().updateInteractionRule(1, {
      participants: [{ sm_id: 1, required_state: 2 }],
    });

    render(<IREditorPanel />);
    expect(screen.getByText(/SM\(1\)/)).toBeInTheDocument();
    expect(screen.getByText(/State 2/)).toBeInTheDocument();
  });

  // --- Guard condition expression editing ---
  it("Guard condition shows ExpressionBuilder", async () => {
    useEditorStore.getState().addInteractionRule();
    useEditorStore.getState().selectInteractionRule(1);
    useEditorStore.getState().updateInteractionRule(1, {
      conditions: [{ kind: "Guard", expr: { Bool: true } }],
    });

    render(<IREditorPanel />);
    // "Guard" appears in the condition label and in the dropdown option
    expect(screen.getAllByText("Guard").length).toBeGreaterThanOrEqual(1);
    // ExpressionBuilder should be present
    expect(screen.getByLabelText("expression type")).toBeInTheDocument();
  });

  it("Guard condition expression can be edited", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().addInteractionRule();
    useEditorStore.getState().selectInteractionRule(1);
    useEditorStore.getState().updateInteractionRule(1, {
      conditions: [{ kind: "Guard", expr: { Bool: true } }],
    });

    render(<IREditorPanel />);

    // Change expression type from Bool to Num
    await user.selectOptions(screen.getByLabelText("expression type"), "Num");

    const rule = useEditorStore.getState().schema.interaction_rules[0];
    expect(rule.conditions[0].kind).toBe("Guard");
    if (rule.conditions[0].kind === "Guard") {
      expect("Num" in rule.conditions[0].expr).toBe(true);
    }
  });

  // --- IR Effect editing ---
  it("shows no effects message initially", async () => {
    const user = userEvent.setup();
    render(<IREditorPanel />);
    await user.click(screen.getByRole("button", { name: /add ir/i }));
    await user.click(screen.getByText(/IR\(1\)/));
    expect(screen.getByText("No effects")).toBeInTheDocument();
  });

  it("Add Effect button adds an effect to the IR", async () => {
    const user = userEvent.setup();
    render(<IREditorPanel />);
    await user.click(screen.getByRole("button", { name: /add ir/i }));
    await user.click(screen.getByText(/IR\(1\)/));
    await user.click(screen.getByText("Add Effect"));

    const rule = useEditorStore.getState().schema.interaction_rules[0];
    expect(rule.effects).toHaveLength(1);
  });

  it("can remove an effect from an IR", async () => {
    useEditorStore.getState().addInteractionRule();
    useEditorStore.getState().selectInteractionRule(1);
    useEditorStore.getState().updateInteractionRule(1, {
      effects: [{ HitStop: { frames: 3 } }],
    });

    const user = userEvent.setup();
    render(<IREditorPanel />);

    await user.click(screen.getByLabelText("remove effect"));
    const rule = useEditorStore.getState().schema.interaction_rules[0];
    expect(rule.effects).toHaveLength(0);
  });
});
