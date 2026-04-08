using System;
using System.IO;
using Xunit;
using Weaven;

namespace Weaven.Tests
{
    /// <summary>
    /// Mycelia demo — C# adapter integration tests.
    ///
    /// Mirrors the Rust tests in weaven-core/tests/mycelia_demo.rs,
    /// verifying the same biological mechanics through the FFI boundary.
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

        private void InitGrass()
        {
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "needs_mycorrhiza", 0.0);
            _world.PushInput(1, "needs_pollinator", 0.0);
            _world.PushInput(1, "hp", 50.0);
            _world.PushInput(1, "carbon", 100.0);
        }

        private void InitOrchid()
        {
            _world.PushInput(1, "moisture_need", 60.0);
            _world.PushInput(1, "light_need", 0.3);
            _world.PushInput(1, "needs_mycorrhiza", 1.0);
            _world.PushInput(1, "needs_pollinator", 1.0);
            _world.PushInput(1, "hp", 30.0);
            _world.PushInput(1, "carbon", 50.0);
        }

        private void GrowPlantTo(int targetState)
        {
            _world.PushInput(1, "moisture", 70.0);
            _world.PushInput(1, "light", 0.8);
            _world.PushInput(1, "growth", 95.0);
            for (int i = 0; i < targetState; i++)
            {
                _world.Activate(1);
                _world.Tick();
            }
        }

        // ── Plant lifecycle ─────────────────────────────────────────

        [Fact]
        public void Plant_SeedDormant_WithoutWater()
        {
            InitGrass();
            _world.PushInput(1, "moisture", 0.0);
            _world.PushInput(1, "light", 1.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(0, _world.ActiveState(1));
        }

        [Fact]
        public void Plant_SeedSprouts_WithConditions()
        {
            InitGrass();
            _world.PushInput(1, "moisture", 30.0);
            _world.PushInput(1, "light", 0.8);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(1));
        }

        [Fact]
        public void Plant_FullLifecycle_SeedToMature()
        {
            InitGrass();
            GrowPlantTo(3);
            Assert.Equal(3, _world.ActiveState(1));
        }

        [Fact]
        public void Plant_WiltsWhenHpZero()
        {
            InitGrass();
            GrowPlantTo(2); // growing
            _world.PushInput(1, "hp", 0.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(5, _world.ActiveState(1)); // wilting
        }

        [Fact]
        public void Plant_DeadAfterWilting()
        {
            InitGrass();
            GrowPlantTo(2);
            _world.PushInput(1, "hp", 0.0);
            _world.Activate(1);
            _world.Tick(); // → wilting
            _world.Activate(1);
            _world.Tick(); // → dead
            Assert.Equal(6, _world.ActiveState(1));
        }

        // ── Orchid win condition ─────────────────────────────────────

        [Fact]
        public void Orchid_BlockedWithoutMycorrhiza()
        {
            InitOrchid();
            GrowPlantTo(3); // mature
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(3, _world.ActiveState(1));
        }

        [Fact]
        public void Orchid_BlockedWithoutPollinator()
        {
            InitOrchid();
            GrowPlantTo(3);
            _world.PushInput(1, "has_mycorrhiza", 1.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(3, _world.ActiveState(1));
        }

        [Fact]
        public void Orchid_Flowers_WithBothRequirements()
        {
            InitOrchid();
            GrowPlantTo(3);
            _world.PushInput(1, "has_mycorrhiza", 1.0);
            _world.PushInput(1, "has_pollinator", 1.0);
            _world.Activate(1);
            _world.Tick();
            Assert.Equal(4, _world.ActiveState(1)); // WIN!
        }

        [Fact]
        public void Grass_FlowersWithoutSpecialRequirements()
        {
            InitGrass();
            GrowPlantTo(4);
            Assert.Equal(4, _world.ActiveState(1));
        }

        // ── Fungus lifecycle ────────────────────────────────────────

        [Fact]
        public void Fungus_GerminatesWithMoisture()
        {
            _world.PushInput(2, "moisture", 50.0);
            _world.Activate(2);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(2));
        }

        [Fact]
        public void Fungus_DormantInDrySoil()
        {
            _world.PushInput(2, "moisture", 20.0);
            _world.Activate(2);
            _world.Tick();
            Assert.Equal(0, _world.ActiveState(2));
        }

        [Fact]
        public void Fungus_FullLifecycleToConnected()
        {
            _world.PushInput(2, "moisture", 50.0);
            _world.PushInput(2, "phosphorus_pool", 30.0);
            _world.PushInput(2, "nitrogen_pool", 20.0);
            _world.Activate(2);
            _world.Tick(); // germinating

            _world.PushInput(2, "growth", 55.0);
            _world.Activate(2);
            _world.Tick(); // growing

            _world.PushInput(2, "growth", 85.0);
            _world.PushInput(2, "nearby_plants", 1.0);
            _world.Activate(2);
            _world.Tick(); // connected
            Assert.Equal(3, _world.ActiveState(2));
        }

        // ── Creature behavior ───────────────────────────────────────

        [Fact]
        public void Creature_ForagesWhenHungry()
        {
            _world.PushInput(3, "hunger", 60.0);
            _world.Activate(3);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(3));
        }

        [Fact]
        public void Creature_FleesOverridesHunger()
        {
            _world.PushInput(3, "hunger", 60.0);
            _world.PushInput(3, "threat_level", 5.0);
            _world.Activate(3);
            _world.Tick();
            Assert.Equal(3, _world.ActiveState(3));
        }

        // ── Soil cell ───────────────────────────────────────────────

        [Fact]
        public void Soil_BarrenToPoor()
        {
            _world.PushInput(4, "organic_matter", 25.0);
            _world.Activate(4);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(4));
        }

        [Fact]
        public void Soil_FullEnrichment()
        {
            _world.PushInput(4, "organic_matter", 85.0);
            _world.PushInput(4, "nutrients", 65.0);
            _world.PushInput(4, "moisture", 50.0);
            for (int i = 0; i < 3; i++)
            {
                _world.Activate(4);
                _world.Tick();
            }
            Assert.Equal(3, _world.ActiveState(4));
        }

        // ── Snapshot/Restore ────────────────────────────────────────

        [Fact]
        public void Snapshot_PreservesAndRestoresState()
        {
            InitGrass();
            GrowPlantTo(2);
            Assert.Equal(2, _world.ActiveState(1));

            var snap = _world.TakeSnapshot();

            _world.PushInput(1, "growth", 95.0);
            _world.Activate(1);
            _world.Tick(); // grows past state 2
            Assert.NotEqual(2, _world.ActiveState(1));

            _world.RestoreSnapshot(snap);
            Assert.Equal(2, _world.ActiveState(1));
        }

        // ── Stability ───────────────────────────────────────────────

        [Fact]
        public void HundredTick_Stability()
        {
            InitGrass();
            _world.PushInput(2, "moisture", 50.0);

            for (int t = 0; t < 100; t++)
            {
                double moisture = 20.0 + 30.0 * Math.Sin(t * 0.1);
                double light = Math.Max(0, Math.Sin(t * 0.05 * Math.PI));
                _world.PushInput(1, "moisture", moisture);
                _world.PushInput(1, "light", light);
                _world.PushInput(2, "moisture", moisture);
                _world.Activate(1);
                _world.Activate(2);
                _world.Activate(3);
                _world.Activate(4);
                _world.Tick();
            }
            // No crash = pass
        }
    }

    internal static class TestContext
    {
        internal static string RepoRoot
        {
            get
            {
                var dir = AppDomain.CurrentDomain.BaseDirectory;
                while (dir != null)
                {
                    if (File.Exists(Path.Combine(dir, "CLAUDE.md")))
                        return dir;
                    dir = Directory.GetParent(dir)?.FullName;
                }
                return Directory.GetCurrentDirectory();
            }
        }
    }
}
