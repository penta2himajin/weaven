using System.IO;
using NUnit.Framework;
using Weaven;

namespace Mycelia.Tests
{
    /// <summary>
    /// Unity EditMode tests for Mycelia demo.
    /// These run in the Unity Editor without entering Play mode.
    /// </summary>
    public class MyceliaEditModeTests
    {
        private WeavenWorld _world;
        private string _schemaJson;

        [SetUp]
        public void SetUp()
        {
            // Try multiple paths to find the schema
            string[] candidates = {
                Path.Combine(UnityEngine.Application.dataPath, "Scripts", "mycelia.json"),
                Path.Combine(UnityEngine.Application.dataPath, "..", "..", "mycelia.json"),
            };

            _schemaJson = null;
            foreach (var path in candidates)
            {
                if (File.Exists(path))
                {
                    _schemaJson = File.ReadAllText(path);
                    break;
                }
            }

            Assert.IsNotNull(_schemaJson, "mycelia.json schema file not found");

            _world = new WeavenWorld();
            _world.LoadSchema(_schemaJson);
        }

        [TearDown]
        public void TearDown()
        {
            _world?.Dispose();
        }

        [Test]
        public void SchemaLoads()
        {
            Assert.IsNotNull(_world);
        }

        [Test]
        public void PlantSeed_StaysDormant_WithoutWater()
        {
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "hp", 50.0);
            _world.PushInput(1, "moisture", 0.0);
            _world.PushInput(1, "light", 1.0);
            _world.Activate(1);
            _world.Tick();
            Assert.AreEqual(0, _world.ActiveState(1));
        }

        [Test]
        public void PlantSeed_Sprouts_WithConditions()
        {
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "hp", 50.0);
            _world.PushInput(1, "moisture", 30.0);
            _world.PushInput(1, "light", 0.8);
            _world.Activate(1);
            _world.Tick();
            Assert.AreEqual(1, _world.ActiveState(1));
        }

        [Test]
        public void GrassFlowers_WithoutSpecialRequirements()
        {
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "needs_mycorrhiza", 0.0);
            _world.PushInput(1, "needs_pollinator", 0.0);
            _world.PushInput(1, "hp", 50.0);
            _world.PushInput(1, "moisture", 30.0);
            _world.PushInput(1, "light", 0.8);
            _world.PushInput(1, "growth", 95.0);

            for (int i = 0; i < 4; i++)
            {
                _world.Activate(1);
                _world.Tick();
            }
            Assert.AreEqual(4, _world.ActiveState(1));
        }

        [Test]
        public void OrchidRequiresMycorrhizaAndPollinator()
        {
            _world.PushInput(1, "moisture_need", 60.0);
            _world.PushInput(1, "light_need", 0.3);
            _world.PushInput(1, "needs_mycorrhiza", 1.0);
            _world.PushInput(1, "needs_pollinator", 1.0);
            _world.PushInput(1, "hp", 30.0);
            _world.PushInput(1, "moisture", 70.0);
            _world.PushInput(1, "light", 0.5);
            _world.PushInput(1, "growth", 95.0);

            // seed -> sprout -> growing -> mature
            for (int i = 0; i < 3; i++) { _world.Activate(1); _world.Tick(); }
            Assert.AreEqual(3, _world.ActiveState(1), "should be mature");

            // Without requirements: stays mature
            _world.Activate(1); _world.Tick();
            Assert.AreEqual(3, _world.ActiveState(1), "blocked without mycorrhiza");

            // Add both -> flowers
            _world.PushInput(1, "has_mycorrhiza", 1.0);
            _world.PushInput(1, "has_pollinator", 1.0);
            _world.Activate(1); _world.Tick();
            Assert.AreEqual(4, _world.ActiveState(1), "orchid blooms!");
        }

        [Test]
        public void FungusGerminates_WithMoisture()
        {
            _world.PushInput(2, "moisture", 50.0);
            _world.Activate(2);
            _world.Tick();
            Assert.AreEqual(1, _world.ActiveState(2));
        }

        [Test]
        public void SeasonTransitions_SpringToSummer()
        {
            _world.PushInput(5, "season_length", 5.0);
            _world.PushInput(5, "tick_in_season", 6.0);
            _world.Activate(5);
            _world.Tick();
            Assert.AreEqual(1, _world.ActiveState(5));
        }

        [Test]
        public void HyphalTipExplores()
        {
            _world.PushInput(6, "moisture", 40.0);
            _world.PushInput(6, "nutrients", 15.0);
            _world.Activate(6);
            _world.Tick();
            Assert.AreEqual(1, _world.ActiveState(6));
        }

        [Test]
        public void BiomeWastelandToPioneer()
        {
            _world.PushInput(7, "soil_quality", 25.0);
            _world.PushInput(7, "plant_cover", 15.0);
            _world.Activate(7);
            _world.Tick();
            Assert.AreEqual(1, _world.ActiveState(7));
        }

        [Test]
        public void SnapshotRestore()
        {
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "hp", 50.0);
            _world.PushInput(1, "moisture", 30.0);
            _world.PushInput(1, "light", 0.8);
            _world.Activate(1); _world.Tick();
            Assert.AreEqual(1, _world.ActiveState(1));

            var snap = _world.TakeSnapshot();

            _world.PushInput(1, "growth", 40.0);
            _world.Activate(1); _world.Tick();
            Assert.AreEqual(2, _world.ActiveState(1));

            _world.RestoreSnapshot(snap);
            Assert.AreEqual(1, _world.ActiveState(1));
        }

        [Test]
        public void HundredTickStability()
        {
            _world.PushInput(1, "moisture_need", 20.0);
            _world.PushInput(1, "light_need", 0.5);
            _world.PushInput(1, "hp", 50.0);
            _world.PushInput(5, "season_length", 10.0);
            _world.PushInput(6, "moisture", 40.0);
            _world.PushInput(6, "nutrients", 15.0);
            _world.PushInput(7, "soil_quality", 30.0);
            _world.PushInput(7, "plant_cover", 15.0);

            for (int t = 0; t < 100; t++)
            {
                double moisture = 20 + 30 * System.Math.Sin(t * 0.1);
                _world.PushInput(1, "moisture", moisture);
                _world.PushInput(1, "light", System.Math.Max(0, System.Math.Sin(t * 0.05 * System.Math.PI)));
                _world.PushInput(2, "moisture", moisture);

                for (uint id = 1; id <= 7; id++) _world.Activate(id);
                _world.Tick();
            }
        }
    }
}
