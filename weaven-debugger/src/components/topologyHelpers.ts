/**
 * Pure functions for TopologyCanvas data transformations.
 * Separated from React components for testability.
 */

import type { HighlightedEdge } from "../stores/debugStore";

export interface RFEdge {
  id: string;
  source: string;
  target: string;
  style?: Record<string, any>;
  animated?: boolean;
  data?: Record<string, any>;
}

/**
 * Apply highlight styles to React Flow edges based on selected trace event.
 *
 * - Signal highlights: matching edge gets cyan + animated, others dim.
 * - Filtered highlights: edge TO the target SM gets red dashed.
 * - No highlights: return edges unchanged.
 */
export function applyHighlights(
  edges: RFEdge[],
  highlights: HighlightedEdge[],
): RFEdge[] {
  if (highlights.length === 0) return edges;

  return edges.map((edge) => {
    const srcId = parseInt(edge.source.replace("sm-", ""), 10);
    const tgtId = parseInt(edge.target.replace("sm-", ""), 10);

    // Check if this edge matches any signal highlight.
    const signalMatch = highlights.find(
      (h) => h.kind === "signal" && h.source === srcId && h.target === tgtId,
    );

    if (signalMatch) {
      return {
        ...edge,
        animated: true,
        style: {
          ...edge.style,
          stroke: "#22d3ee",
          strokeWidth: 3,
          opacity: 1,
        },
      };
    }

    // Check if this edge matches a filtered highlight.
    // PipelineFiltered uses source=target=targetSM, so we match edges TO that SM.
    const filteredMatch = highlights.find(
      (h) => h.kind === "filtered" && h.target === tgtId,
    );

    if (filteredMatch) {
      return {
        ...edge,
        animated: false,
        style: {
          ...edge.style,
          stroke: "#ef4444",
          strokeWidth: 2,
          strokeDasharray: "4 4",
          opacity: 1,
        },
      };
    }

    // Not matched — dim this edge.
    return {
      ...edge,
      animated: false,
      style: {
        ...edge.style,
        opacity: 0.3,
      },
    };
  });
}
