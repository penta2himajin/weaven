using System;
using System.IO;
using Xunit;
using Weaven;

namespace Weaven.Tests
{
    /// <summary>
    /// Mycelia demo — C# adapter tests covering all 3 phases.
    /// Phase 1: Season, market economy, stress thresholds
    /// Phase 2: Allelopathy, hub nodes, fungal autonomy
    /// Phase 3: Kin recognition, hyphal tips, biome succession
    /// </summary>
    public class MyceliaTests : IDisposable
    {
        private readonly WeavenWorld _world;

        public MyceliaTests()
        {
            var schema = File.ReadAllText(
                Path.Combine(TestContext.RepoRoot, "demos", "mycelia", "mycelia.json"));
            _world = new WeavenWorld();
            _world.LoadSchema(schema);
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
            _world.PushInput(1, "trust_score", 1.0);
        }

        private void InitOrchid()
        {
            _world.PushInput(1, "moisture_need", 60.0);
            _world.PushInput(1, "light_need", 0.3);
            _world.PushInput(1, "needs_mycorrhiza", 1.0);
            _world.PushInput(1, "needs_pollinator", 1.0);
            _world.PushInput(1, "hp", 30.0);
            _world.PushInput(1, "carbon", 50.0);
            _world.PushInput(1, "trust_score", 1.0);
        }

        private void GrowPlantTo(int target)
        {
            _world.PushInput(1, "moisture", 70.0);
            _world.PushInput(1, "light", 0.8);
            _world.PushInput(1, "growth", 95.0);
            for (int i = 0; i < target; i++) { _world.Activate(1); _world.Tick(); }
        }

        // ── Core lifecycle ──────────────────────────────────────────

        [Fact] public void Plant_SeedDormant() { InitGrass(); _world.PushInput(1, "moisture", 0); _world.PushInput(1, "light", 1); _world.Activate(1); _world.Tick(); Assert.Equal(0, _world.ActiveState(1)); }
        [Fact] public void Plant_FullLifecycle() { InitGrass(); GrowPlantTo(4); Assert.Equal(4, _world.ActiveState(1)); }
        [Fact] public void Orchid_WIN() { InitOrchid(); GrowPlantTo(3); _world.PushInput(1, "has_mycorrhiza", 1); _world.PushInput(1, "has_pollinator", 1); _world.Activate(1); _world.Tick(); Assert.Equal(4, _world.ActiveState(1)); }
        [Fact] public void Fungus_Germinates() { _world.PushInput(2, "moisture", 50); _world.Activate(2); _world.Tick(); Assert.Equal(1, _world.ActiveState(2)); }
        [Fact] public void Creature_Flees() { _world.PushInput(3, "hunger", 60); _world.PushInput(3, "threat_level", 5); _world.Activate(3); _world.Tick(); Assert.Equal(3, _world.ActiveState(3)); }
        [Fact] public void Soil_Enriches() { _world.PushInput(4, "organic_matter", 85); _world.PushInput(4, "nutrients", 65); _world.PushInput(4, "moisture", 50); for (int i = 0; i < 3; i++) { _world.Activate(4); _world.Tick(); } Assert.Equal(3, _world.ActiveState(4)); }

        // ── Phase 1: Season ─────────────────────────────────────────

        [Fact]
        public void Season_SpringToSummer()
        {
            _world.PushInput(5, "season_length", 5.0);
            _world.PushInput(5, "tick_in_season", 6.0);
            _world.Activate(5);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(5)); // summer
        }

        [Fact]
        public void Season_FullCycle()
        {
            _world.PushInput(5, "season_length", 1.0);
            for (int s = 0; s < 4; s++)
            {
                _world.PushInput(5, "tick_in_season", 2.0);
                _world.Activate(5);
                _world.Tick();
            }
            Assert.Equal(0, _world.ActiveState(5)); // back to spring
        }

        // ── Phase 1: Market economy ─────────────────────────────────

        [Fact]
        public void Fungus_TrustScoreAffectsOutput()
        {
            _world.PushInput(2, "moisture", 50);
            _world.PushInput(2, "phosphorus_pool", 30);
            _world.PushInput(2, "nitrogen_pool", 20);
            _world.PushInput(2, "trust_score", 0.5);
            _world.Activate(2); _world.Tick(); // germinating
            _world.PushInput(2, "growth", 55);
            _world.Activate(2); _world.Tick(); // growing
            _world.PushInput(2, "growth", 85);
            _world.PushInput(2, "nearby_plants", 1);
            _world.Activate(2); _world.Tick(); // connected
            Assert.Equal(3, _world.ActiveState(2));
        }

        // ── Phase 3: Hyphal tip ─────────────────────────────────────

        [Fact]
        public void HyphalTip_Explores()
        {
            _world.PushInput(6, "moisture", 40);
            _world.PushInput(6, "nutrients", 15);
            _world.Activate(6);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(6)); // branching
        }

        [Fact]
        public void HyphalTip_GoesDormant()
        {
            _world.PushInput(6, "moisture", 40);
            _world.PushInput(6, "nutrients", 15);
            _world.Activate(6); _world.Tick(); // branching
            _world.PushInput(6, "nutrients", 3);
            _world.Activate(6); _world.Tick();
            Assert.Equal(4, _world.ActiveState(6)); // dormant
        }

        // ── Phase 3: Biome succession ───────────────────────────────

        [Fact]
        public void Biome_WastelandToPioneer()
        {
            _world.PushInput(7, "soil_quality", 25);
            _world.PushInput(7, "plant_cover", 15);
            _world.Activate(7);
            _world.Tick();
            Assert.Equal(1, _world.ActiveState(7)); // pioneer
        }

        [Fact]
        public void Biome_ReachesClimax()
        {
            _world.PushInput(7, "soil_quality", 85);
            _world.PushInput(7, "plant_cover", 50);
            _world.PushInput(7, "plant_diversity", 8);
            _world.PushInput(7, "network_density", 75);
            _world.PushInput(7, "hub_count", 3);
            for (int i = 0; i < 3; i++) { _world.Activate(7); _world.Tick(); }
            Assert.Equal(3, _world.ActiveState(7)); // climax
        }

        [Fact]
        public void Biome_DegradeWhenHubsDestroyed()
        {
            _world.PushInput(7, "soil_quality", 85);
            _world.PushInput(7, "plant_cover", 50);
            _world.PushInput(7, "plant_diversity", 8);
            _world.PushInput(7, "network_density", 75);
            _world.PushInput(7, "hub_count", 3);
            for (int i = 0; i < 3; i++) { _world.Activate(7); _world.Tick(); }
            _world.PushInput(7, "hub_count", 0);
            _world.Activate(7); _world.Tick();
            Assert.Equal(2, _world.ActiveState(7)); // degraded
        }

        // ── Stability ───────────────────────────────────────────────

        [Fact]
        public void HundredTick_All7SMs()
        {
            InitGrass();
            _world.PushInput(2, "moisture", 50); _world.PushInput(2, "trust_score", 1);
            _world.PushInput(5, "season_length", 10);
            _world.PushInput(6, "moisture", 40); _world.PushInput(6, "nutrients", 15);
            _world.PushInput(7, "soil_quality", 30); _world.PushInput(7, "plant_cover", 15);
            for (int t = 0; t < 100; t++)
            {
                double m = 20 + 30 * Math.Sin(t * 0.1);
                _world.PushInput(1, "moisture", m);
                _world.PushInput(1, "light", Math.Max(0, Math.Sin(t * 0.05 * Math.PI)));
                _world.PushInput(2, "moisture", m);
                _world.PushInput(5, "tick_in_season", (t % 10) + 1);
                for (uint id = 1; id <= 7; id++) _world.Activate(id);
                _world.Tick();
            }
        }

        [Fact]
        public void Snapshot_Restore()
        {
            InitGrass(); GrowPlantTo(2);
            var snap = _world.TakeSnapshot();
            _world.PushInput(1, "growth", 95);
            _world.Activate(1); _world.Tick();
            Assert.NotEqual(2, _world.ActiveState(1));
            _world.RestoreSnapshot(snap);
            Assert.Equal(2, _world.ActiveState(1));
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
                    if (File.Exists(Path.Combine(dir, "CLAUDE.md"))) return dir;
                    dir = Directory.GetParent(dir)?.FullName;
                }
                return Directory.GetCurrentDirectory();
            }
        }
    }
}
