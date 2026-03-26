import { describe, it, expect } from "vitest";
import { applyHighlights, type RFEdge } from "../components/topologyHelpers";
import type { HighlightedEdge } from "../stores/debugStore";

const baseEdges: RFEdge[] = [
  { id: "edge-0", source: "sm-1", target: "sm-2", style: { stroke: "#6366f1" }, animated: false, data: {} },
  { id: "edge-1", source: "sm-2", target: "sm-3", style: { stroke: "#6366f1" }, animated: false, data: {} },
];

describe("applyHighlights", () => {
  it("returns edges unchanged when no highlights", () => {
    const result = applyHighlights(baseEdges, []);
    expect(result[0].style?.stroke).toBe("#6366f1");
    expect(result[0].animated).toBe(false);
  });

  it("highlights matching signal edge with animation", () => {
    const highlights: HighlightedEdge[] = [{ source: 1, target: 2, kind: "signal" }];
    const result = applyHighlights(baseEdges, highlights);

    // Edge sm-1→sm-2 should be highlighted.
    expect(result[0].animated).toBe(true);
    expect(result[0].style?.stroke).toBe("#22d3ee"); // cyan highlight
    expect(result[0].style?.strokeWidth).toBe(3);

    // Edge sm-2→sm-3 should be dimmed.
    expect(result[1].animated).toBe(false);
    expect(result[1].style?.opacity).toBe(0.3);
  });

  it("marks filtered edge with red dashed style", () => {
    const highlights: HighlightedEdge[] = [{ source: 3, target: 3, kind: "filtered" }];
    const result = applyHighlights(baseEdges, highlights);

    // No edge directly matches sm-3→sm-3, but edges TO sm-3 should be marked.
    const toSm3 = result.find((e) => e.target === "sm-3");
    expect(toSm3?.style?.stroke).toBe("#ef4444"); // red for filtered
    expect(toSm3?.style?.strokeDasharray).toBe("4 4");
  });

  it("dims non-highlighted edges when highlights are active", () => {
    const highlights: HighlightedEdge[] = [{ source: 1, target: 2, kind: "signal" }];
    const result = applyHighlights(baseEdges, highlights);

    // Edge 1 (sm-2→sm-3) should be dimmed.
    expect(result[1].style?.opacity).toBe(0.3);
  });
});
