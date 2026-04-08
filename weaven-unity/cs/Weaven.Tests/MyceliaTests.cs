using System;
using System.IO;
using Xunit;
using Weaven;

namespace Weaven.Tests
{
    /// <summary>
    /// Mycelia demo — C# adapter integration tests.
    ///
    /// Tests the Weaven C# adapter (WeavenWorld) with the Mycelia schema,
    /// verifying that the state machine transitions, context manipulation,
    /// and tick simulation work correctly through the FFI boundary.
    /// </summary>
    public class MyceliaTests : IDisposable
    {
        private readonly WeavenWorld _world;
        private readonly string _schema;

        public MyceliaTests()
        {
            _schema = File.ReadAllText(
                Path.Combine(TestContext.RepoRoot, "demos", "mycelia", "mycelia.json"));
            _world = new WeavenWorld();
            _world.LoadSchema(_schema);
        }

        public void Dispose() => _world.Dispose();

        // ── Plant SM (id=1) ─────────────────────────────────────────────

        [Fact]
        public void PlantSeed_StaysDormant_WithoutWater()
        {
            // Plant SM starts in seed state (0)
            // Set grass requirements but no moisture
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "moisture", 0.0);
            _world.PushInput(1, "light", 1.0);
            _world.PushInput(1, "hp", 50.0);

            _world.Activate(1);
            var result = _world.Tick();

            Assert.Equal(0, _world.ActiveState(1));
        }

        [Fact]
        public void PlantSeed_Sprouts_WithSufficientConditions()
        {
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "moisture", 30.0);
            _world.PushInput(1, "light", 0.8);
            _world.PushInput(1, "hp", 50.0);

            _world.Activate(1);
            _world.Tick();

            Assert.Equal(1, _world.ActiveState(1));
        }

        [Fact]
        public void PlantGrowth_ProgressesToMature()
        {
            // Seed -> Sprout
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "moisture", 30.0);
            _world.PushInput(1, "light", 0.8);
            _world.PushInput(1, "hp", 50.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(1));

            // Sprout -> Growing
            _world.PushInput(1, "growth", 35.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(2, _world.ActiveState(1));

            // Growing -> Mature
            _world.PushInput(1, "growth", 75.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(3, _world.ActiveState(1));
        }

        // ── Orchid Win Condition ─────────────────────────────────────────

        [Fact]
        public void Orchid_CannotFlower_WithoutMycorrhiza()
        {
            // Set orchid requirements
            _world.PushInput(1, "moisture_need", 60.0);
            _world.PushInput(1, "light_need", 0.3);
            _world.PushInput(1, "needs_mycorrhiza", 1.0);
            _world.PushInput(1, "needs_pollinator", 1.0);
            _world.PushInput(1, "moisture", 70.0);
            _world.PushInput(1, "light", 0.5);
            _world.PushInput(1, "hp", 30.0);
            _world.PushInput(1, "growth", 95.0);

            // Progress: seed -> sprout -> growing -> mature
            for (int i = 0; i < 3; i++)
            {
                _world.Activate(1);
                _world.Tick();
            }
            Assert.Equal(3, _world.ActiveState(1)); // Mature

            // Try to flower without mycorrhiza
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(3, _world.ActiveState(1)); // Still mature
        }

        [Fact]
        public void Orchid_Flowers_WithBothMycorrhizaAndPollinator()
        {
            _world.PushInput(1, "moisture_need", 60.0);
            _world.PushInput(1, "light_need", 0.3);
            _world.PushInput(1, "needs_mycorrhiza", 1.0);
            _world.PushInput(1, "needs_pollinator", 1.0);
            _world.PushInput(1, "moisture", 70.0);
            _world.PushInput(1, "light", 0.5);
            _world.PushInput(1, "hp", 30.0);
            _world.PushInput(1, "growth", 95.0);

            // seed -> sprout -> growing -> mature
            for (int i = 0; i < 3; i++)
            {
                _world.Activate(1);
                _world.Tick();
            }
            Assert.Equal(3, _world.ActiveState(1));

            // Add both requirements
            _world.PushInput(1, "has_mycorrhiza", 1.0);
            _world.PushInput(1, "has_pollinator", 1.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(4, _world.ActiveState(1)); // Flowering! WIN!
        }

        // ── Fungus SM (id=2) ────────────────────────────────────────────

        [Fact]
        public void Fungus_GerminatesWithMoisture()
        {
            _world.PushInput(2, "moisture", 50.0);
            _world.Activate(2);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(2));
        }

        [Fact]
        public void Fungus_StaysDormantInDrySoil()
        {
            _world.PushInput(2, "moisture", 20.0);
            _world.Activate(2);
            _world.Tick();
            Assert.Equal(0, _world.ActiveState(2));
        }

        // ── Creature SM (id=3) ──────────────────────────────────────────

        [Fact]
        public void Creature_FleesWhenThreatened()
        {
            _world.PushInput(3, "hunger", 60.0);
            _world.PushInput(3, "threat_level", 5.0);
            _world.Activate(3);
            _world.Tick();
            Assert.Equal(3, _world.ActiveState(3)); // Fleeing
        }

        // ── Soil Cell SM (id=4) ─────────────────────────────────────────

        [Fact]
        public void Soil_EnrichesWithOrganicMatter()
        {
            _world.PushInput(4, "organic_matter", 25.0);
            _world.Activate(4);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(4)); // barren -> poor
        }

        // ── Snapshot/Restore ────────────────────────────────────────────

        [Fact]
        public void Snapshot_PreservesAndRestoresState()
        {
            // Advance plant to sprout
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "moisture", 30.0);
            _world.PushInput(1, "light", 0.8);
            _world.PushInput(1, "hp", 50.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(1));

            // Take snapshot
            var snap = _world.TakeSnapshot();
            Assert.False(string.IsNullOrEmpty(snap));

            // Advance further
            _world.PushInput(1, "growth", 40.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(2, _world.ActiveState(1)); // Growing

            // Restore -> back to sprout
            _world.RestoreSnapshot(snap);
            Assert.Equal(1, _world.ActiveState(1));
        }
    }

    /// <summary>
    /// Helper to locate the repo root for loading schema files.
    /// </summary>
    internal static class TestContext
    {
        internal static string RepoRoot
        {
            get
            {
                // Walk up from bin/Debug/net8.0 to find the repo root
                var dir = AppDomain.CurrentDomain.BaseDirectory;
                while (dir != null)
                {
                    if (File.Exists(Path.Combine(dir, "CLAUDE.md")))
                        return dir;
                    dir = Directory.GetParent(dir)?.FullName;
                }
                // Fallback: assume running from repo root
                return Directory.GetCurrentDirectory();
            }
        }
    }
}
