using System.Collections.Generic;
using Xunit;
using Weaven.Generated;
using Weaven.Generated.Validation;

namespace Weaven.Tests
{
    public class ValidatorsTests
    {
        // ── NoSelfLoop ──────────────────────────────────────────────────

        [Fact]
        public void ValidateNoSelfLoop_DifferentNodes_ReturnsTrue()
        {
            var src = new GraphNode(new SmId(1), new StateId(0));
            var tgt = new GraphNode(new SmId(2), new StateId(0));
            var edge = new GraphEdge(src, tgt, EdgeKind.EdgeStatic, null);
            Assert.True(Validators.ValidateNoSelfLoop(edge));
        }

        [Fact]
        public void ValidateNoSelfLoop_SameNode_ReturnsFalse()
        {
            var node = new GraphNode(new SmId(1), new StateId(0));
            var edge = new GraphEdge(node, node, EdgeKind.EdgeStatic, null);
            Assert.False(Validators.ValidateNoSelfLoop(edge));
        }

        // ── EdgesReferenceGraphNodes ────────────────────────────────────

        [Fact]
        public void ValidateEdgesReferenceGraphNodes_ValidGraph_ReturnsTrue()
        {
            var n1 = new GraphNode(new SmId(1), new StateId(0));
            var n2 = new GraphNode(new SmId(2), new StateId(0));
            var edge = new GraphEdge(n1, n2, EdgeKind.EdgeStatic, null);
            var graph = new TopologyGraph(
                new HashSet<GraphNode> { n1, n2 },
                new HashSet<GraphEdge> { edge });
            Assert.True(Validators.ValidateEdgesReferenceGraphNodes(graph));
        }

        [Fact]
        public void ValidateEdgesReferenceGraphNodes_OrphanEdge_ReturnsFalse()
        {
            var n1 = new GraphNode(new SmId(1), new StateId(0));
            var n2 = new GraphNode(new SmId(2), new StateId(0));
            var n3 = new GraphNode(new SmId(3), new StateId(0));
            var edge = new GraphEdge(n1, n3, EdgeKind.EdgeStatic, null);
            // n3 is NOT in the node set
            var graph = new TopologyGraph(
                new HashSet<GraphNode> { n1, n2 },
                new HashSet<GraphEdge> { edge });
            Assert.False(Validators.ValidateEdgesReferenceGraphNodes(graph));
        }

        [Fact]
        public void ValidateEdgesReferenceGraphNodes_EmptyGraph_ReturnsTrue()
        {
            var graph = new TopologyGraph(
                new HashSet<GraphNode>(),
                new HashSet<GraphEdge>());
            Assert.True(Validators.ValidateEdgesReferenceGraphNodes(graph));
        }

        // ── UniqueSmPerNode ─────────────────────────────────────────────

        [Fact]
        public void ValidateUniqueSmPerNode_AllUnique_ReturnsTrue()
        {
            var n1 = new GraphNode(new SmId(1), new StateId(0));
            var n2 = new GraphNode(new SmId(2), new StateId(1));
            var graph = new TopologyGraph(
                new HashSet<GraphNode> { n1, n2 },
                new HashSet<GraphEdge>());
            Assert.True(Validators.ValidateUniqueSmPerNode(graph));
        }

        [Fact]
        public void ValidateUniqueSmPerNode_DuplicateSm_ReturnsFalse()
        {
            // Same SmId(1) but different ActiveState → different GraphNode instances
            var n1 = new GraphNode(new SmId(1), new StateId(0));
            var n2 = new GraphNode(new SmId(1), new StateId(1));
            // Force both into the set (they are different record instances)
            var nodes = new HashSet<GraphNode>(ReferenceEqualityComparer.Instance) { n1, n2 };
            var graph = new TopologyGraph(nodes, new HashSet<GraphEdge>());
            Assert.False(Validators.ValidateUniqueSmPerNode(graph));
        }

        // ── NoCyclicEvalTree ────────────────────────────────────────────

        [Fact]
        public void ValidateNoCyclicEvalTree_SimpleTree_ReturnsTrue()
        {
            var leaf = new EvalTreeNode(ExprKind.ExprLit, new Label("1"), new EvalValue(1.0), new List<EvalTreeNode>());
            var root = new EvalTreeNode(ExprKind.ExprBinOp, new Label("+"), new EvalValue(2.0), new List<EvalTreeNode> { leaf });
            Assert.True(Validators.ValidateNoCyclicEvalTree(root));
        }

        [Fact]
        public void ValidateNoCyclicEvalTree_SingleNode_ReturnsTrue()
        {
            var node = new EvalTreeNode(ExprKind.ExprLit, new Label("x"), new EvalValue(0.0), new List<EvalTreeNode>());
            Assert.True(Validators.ValidateNoCyclicEvalTree(node));
        }

        // ── CursorRange ─────────────────────────────────────────────────

        [Fact]
        public void ValidateCursorRange_Valid_ReturnsTrue()
        {
            var cursor = new TickCursor(5, 10);
            Assert.True(Validators.ValidateCursorRange(cursor));
        }

        [Fact]
        public void ValidateCursorRange_Equal_ReturnsTrue()
        {
            var cursor = new TickCursor(10, 10);
            Assert.True(Validators.ValidateCursorRange(cursor));
        }

        [Fact]
        public void ValidateCursorRange_CurrentExceedsMax_ReturnsFalse()
        {
            var cursor = new TickCursor(11, 10);
            Assert.False(Validators.ValidateCursorRange(cursor));
        }

        // ── MaxTickNonNeg ───────────────────────────────────────────────

        [Fact]
        public void ValidateMaxTickNonNeg_Positive_ReturnsTrue()
        {
            var cursor = new TickCursor(0, 10);
            Assert.True(Validators.ValidateMaxTickNonNeg(cursor));
        }

        [Fact]
        public void ValidateMaxTickNonNeg_Zero_ReturnsTrue()
        {
            var cursor = new TickCursor(0, 0);
            Assert.True(Validators.ValidateMaxTickNonNeg(cursor));
        }

        [Fact]
        public void ValidateMaxTickNonNeg_Negative_ReturnsFalse()
        {
            var cursor = new TickCursor(0, -1);
            Assert.False(Validators.ValidateMaxTickNonNeg(cursor));
        }

        // ── SnapshotNonEmpty ────────────────────────────────────────────

        [Fact]
        public void ValidateSnapshotNonEmpty_NonEmpty_ReturnsTrue()
        {
            var session = new DebugSession(
                new List<WorldSnapshot> { new WorldSnapshot() },
                new TickCursor(0, 0),
                null,
                new List<TraceEvent>(),
                new TopologyGraph(new HashSet<GraphNode>(), new HashSet<GraphEdge>()),
                new FilterConfig(new HashSet<SmId>(), new HashSet<ConnectionId>(), new HashSet<Phase>()));
            Assert.True(Validators.ValidateSnapshotNonEmpty(session));
        }

        [Fact]
        public void ValidateSnapshotNonEmpty_Empty_ReturnsFalse()
        {
            var session = new DebugSession(
                new List<WorldSnapshot>(),
                new TickCursor(0, 0),
                null,
                new List<TraceEvent>(),
                new TopologyGraph(new HashSet<GraphNode>(), new HashSet<GraphEdge>()),
                new FilterConfig(new HashSet<SmId>(), new HashSet<ConnectionId>(), new HashSet<Phase>()));
            Assert.False(Validators.ValidateSnapshotNonEmpty(session));
        }
    }
}
