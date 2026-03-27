import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import GuardEvalTree from "../components/GuardEvalTree";
import type { EvalTreeNode } from "../generated/models";

describe("GuardEvalTree", () => {
  it("renders a leaf node", () => {
    const tree: EvalTreeNode = {
      exprKind: "CtxRef",
      label: "context.hp",
      value: 10,
      children: [],
    };
    render(<GuardEvalTree tree={tree} />);
    expect(screen.getByText("context.hp")).toBeInTheDocument();
    expect(screen.getByText("= 10")).toBeInTheDocument();
    expect(screen.getByTestId("guard-eval-tree")).toBeInTheDocument();
  });

  it("renders a binary op with children", () => {
    const tree: EvalTreeNode = {
      exprKind: "BinOp",
      label: ">",
      value: 1,
      children: [
        { exprKind: "CtxRef", label: "context.hp", value: 10, children: [] },
        { exprKind: "Lit", label: "0", value: 0, children: [] },
      ],
    };
    render(<GuardEvalTree tree={tree} />);
    expect(screen.getByText(">")).toBeInTheDocument();
    expect(screen.getByText("context.hp")).toBeInTheDocument();
    expect(screen.getByText("= true")).toBeInTheDocument();
  });

  it("shows pass/fail indicators", () => {
    const tree: EvalTreeNode = {
      exprKind: "BinOp",
      label: "==",
      value: 0, // false
      children: [
        { exprKind: "CtxRef", label: "context.x", value: 5, children: [] },
        { exprKind: "Lit", label: "10", value: 10, children: [] },
      ],
    };
    render(<GuardEvalTree tree={tree} />);
    // Root evaluates to 0 (false) — should show cross mark
    expect(screen.getByText("= false")).toBeInTheDocument();
  });
});
