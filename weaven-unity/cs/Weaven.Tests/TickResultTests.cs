using Xunit;
using Weaven;

namespace Weaven.Tests
{
    public class TickResultTests
    {
        [Fact]
        public void FromJson_EmptyObject_ReturnsEmptyChanges()
        {
            var result = TickResult.FromJson("{}");
            Assert.Empty(result.StateChanges);
            Assert.Empty(result.SystemCommands);
            Assert.Equal(0UL, result.Tick);
        }

        [Fact]
        public void FromJson_SingleStateChange_ParsesCorrectly()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{\"1\":[0,1]},\"system_commands\":[],\"tick\":5}");
            Assert.Single(result.StateChanges);
            Assert.Equal((0, 1), result.StateChanges[1]);
            Assert.Equal(5UL, result.Tick);
        }

        [Fact]
        public void FromJson_MultipleStateChanges_ParsesAll()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{\"1\":[0,1],\"2\":[1,0],\"3\":[2,3]},\"system_commands\":[],\"tick\":42}");
            Assert.Equal(3, result.StateChanges.Count);
            Assert.Equal((0, 1), result.StateChanges[1]);
            Assert.Equal((1, 0), result.StateChanges[2]);
            Assert.Equal((2, 3), result.StateChanges[3]);
            Assert.Equal(42UL, result.Tick);
        }

        [Fact]
        public void FromJson_TickOnly_NoStateChanges()
        {
            var result = TickResult.FromJson("{\"tick\":100}");
            Assert.Empty(result.StateChanges);
            Assert.Equal(100UL, result.Tick);
        }

        [Fact]
        public void FromJson_WhitespaceInTick_HandledCorrectly()
        {
            var result = TickResult.FromJson("{\"tick\": 7 }");
            Assert.Equal(7UL, result.Tick);
        }

        [Fact]
        public void FromJson_LargeSmId_ParsesCorrectly()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{\"4294967295\":[0,1]},\"tick\":0}");
            Assert.Equal((0, 1), result.StateChanges[uint.MaxValue]);
        }

        [Fact]
        public void FromJson_EmptyStateChanges_ReturnsEmptyDict()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{},\"system_commands\":[],\"tick\":1}");
            Assert.Empty(result.StateChanges);
            Assert.Equal(1UL, result.Tick);
        }

        // ── SystemCommand parsing ────────────────────────────────────────

        [Fact]
        public void FromJson_HitStopCommand_Parsed()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{},\"system_commands\":[{\"HitStop\":{\"frames\":3}}],\"tick\":1}");
            Assert.Single(result.SystemCommands);
            var cmd = Assert.IsType<SystemCommand.HitStop>(result.SystemCommands[0]);
            Assert.Equal(3U, cmd.Frames);
        }

        [Fact]
        public void FromJson_SlowMotionCommand_Parsed()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{},\"system_commands\":[{\"SlowMotion\":{\"factor\":0.5,\"duration_ticks\":10}}],\"tick\":1}");
            Assert.Single(result.SystemCommands);
            var cmd = Assert.IsType<SystemCommand.SlowMotion>(result.SystemCommands[0]);
            Assert.Equal(0.5, cmd.Factor);
            Assert.Equal(10U, cmd.DurationTicks);
        }

        [Fact]
        public void FromJson_TimeScaleCommand_Parsed()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{},\"system_commands\":[{\"TimeScale\":1.5}],\"tick\":1}");
            Assert.Single(result.SystemCommands);
            var cmd = Assert.IsType<SystemCommand.TimeScale>(result.SystemCommands[0]);
            Assert.Equal(1.5, cmd.Scale);
        }

        [Fact]
        public void FromJson_MultipleCommands_ParsedInOrder()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{},\"system_commands\":[{\"HitStop\":{\"frames\":5}},{\"TimeScale\":2.0}],\"tick\":1}");
            Assert.Equal(2, result.SystemCommands.Count);
            Assert.IsType<SystemCommand.HitStop>(result.SystemCommands[0]);
            Assert.IsType<SystemCommand.TimeScale>(result.SystemCommands[1]);
        }

        [Fact]
        public void FromJson_EmptySystemCommands_ReturnsEmptyList()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{},\"system_commands\":[],\"tick\":1}");
            Assert.Empty(result.SystemCommands);
        }

        [Fact]
        public void FromJson_StateChangesWithCommands_BothParsed()
        {
            var result = TickResult.FromJson(
                "{\"state_changes\":{\"1\":[0,1]},\"system_commands\":[{\"HitStop\":{\"frames\":2}}],\"tick\":10}");
            Assert.Single(result.StateChanges);
            Assert.Equal((0, 1), result.StateChanges[1]);
            Assert.Single(result.SystemCommands);
            Assert.Equal(10UL, result.Tick);
        }
    }
}
