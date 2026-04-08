using System.IO;
using UnityEngine;
using Weaven;

/// <summary>
/// Headless test runner for Mycelia.
/// Runs a complete simulation scenario and validates the orchid bloom
/// win condition without any rendering or user input.
///
/// Usage (batchmode):
///   Unity -batchmode -nographics -projectPath . \
///         -executeMethod MyceliaTestRunner.RunHeadless -quit
/// </summary>
public static class MyceliaTestRunner
{
    public static void RunHeadless()
    {
        Debug.Log("[MyceliaTest] Starting headless test run...");

        string schemaPath = Path.Combine(Application.dataPath, "..", "..", "mycelia.json");
        if (!File.Exists(schemaPath))
        {
            Debug.LogError($"[MyceliaTest] Schema not found at {schemaPath}");
            UnityEngine.Application.Quit(1);
            return;
        }

        string json = File.ReadAllText(schemaPath);
        int failures = 0;

        // Test 1: Schema loads
        failures += Test("Schema loads", () =>
        {
            using var world = new WeavenWorld();
            world.LoadSchema(json);
            return true;
        });

        // Test 2: Grass lifecycle (seed -> sprout -> growing -> mature -> flowering)
        failures += Test("Grass full lifecycle", () =>
        {
            using var world = new WeavenWorld();
            world.LoadSchema(json);

            // Init grass parameters
            world.PushInput(1, "moisture_need", 20.0);
            world.PushInput(1, "light_need", 0.5);
            world.PushInput(1, "hp", 50.0);
            world.PushInput(1, "moisture", 30.0);
            world.PushInput(1, "light", 0.8);
            world.PushInput(1, "needs_mycorrhiza", 0.0);
            world.PushInput(1, "needs_pollinator", 0.0);
            world.PushInput(1, "growth", 95.0);

            // Tick 4 times: seed -> sprout -> growing -> mature -> flowering
            for (int i = 0; i < 4; i++)
            {
                world.Activate(1);
                world.Tick();
            }

            return world.ActiveState(1) == 4; // flowering
        });

        // Test 3: Orchid requires mycorrhiza + pollinator
        failures += Test("Orchid win condition", () =>
        {
            using var world = new WeavenWorld();
            world.LoadSchema(json);

            world.PushInput(1, "moisture_need", 60.0);
            world.PushInput(1, "light_need", 0.3);
            world.PushInput(1, "needs_mycorrhiza", 1.0);
            world.PushInput(1, "needs_pollinator", 1.0);
            world.PushInput(1, "moisture", 70.0);
            world.PushInput(1, "light", 0.5);
            world.PushInput(1, "hp", 30.0);
            world.PushInput(1, "growth", 95.0);

            // seed -> sprout -> growing -> mature
            for (int i = 0; i < 3; i++) { world.Activate(1); world.Tick(); }
            if (world.ActiveState(1) != 3) return false; // Should be mature

            // Without mycorrhiza, stays mature
            world.Activate(1); world.Tick();
            if (world.ActiveState(1) != 3) return false;

            // Add both requirements
            world.PushInput(1, "has_mycorrhiza", 1.0);
            world.PushInput(1, "has_pollinator", 1.0);
            world.Activate(1); world.Tick();

            return world.ActiveState(1) == 4; // flowering = WIN
        });

        // Test 4: Fungus lifecycle
        failures += Test("Fungus germination", () =>
        {
            using var world = new WeavenWorld();
            world.LoadSchema(json);

            world.PushInput(2, "moisture", 50.0);
            world.Activate(2);
            world.Tick();

            return world.ActiveState(2) == 1; // germinating
        });

        // Test 5: Snapshot round-trip
        failures += Test("Snapshot preserves state", () =>
        {
            using var world = new WeavenWorld();
            world.LoadSchema(json);

            world.PushInput(1, "moisture_need", 20.0);
            world.PushInput(1, "light_need", 0.5);
            world.PushInput(1, "moisture", 30.0);
            world.PushInput(1, "light", 0.8);
            world.PushInput(1, "hp", 50.0);
            world.Activate(1);
            world.Tick();
            if (world.ActiveState(1) != 1) return false; // sprout

            var snap = world.TakeSnapshot();
            world.PushInput(1, "growth", 40.0);
            world.Activate(1);
            world.Tick();
            if (world.ActiveState(1) != 2) return false; // growing

            world.RestoreSnapshot(snap);
            return world.ActiveState(1) == 1; // back to sprout
        });

        // Test 6: 100-tick stability
        failures += Test("100-tick simulation stability", () =>
        {
            using var world = new WeavenWorld();
            world.LoadSchema(json);

            world.PushInput(1, "moisture_need", 20.0);
            world.PushInput(1, "light_need", 0.5);
            world.PushInput(1, "hp", 50.0);

            for (int t = 0; t < 100; t++)
            {
                double moisture = 20 + 30 * System.Math.Sin(t * 0.1);
                double light = System.Math.Max(0, System.Math.Sin(t * 0.05 * System.Math.PI));
                world.PushInput(1, "moisture", moisture);
                world.PushInput(1, "light", light);
                world.PushInput(2, "moisture", moisture);
                world.Activate(1);
                world.Activate(2);
                world.Activate(3);
                world.Activate(4);
                world.Tick();
            }
            return true; // No crash = pass
        });

        Debug.Log($"[MyceliaTest] Results: {(failures == 0 ? "ALL PASSED" : $"{failures} FAILED")}");
        UnityEngine.Application.Quit(failures);
    }

    private static int Test(string name, System.Func<bool> testFn)
    {
        try
        {
            bool passed = testFn();
            Debug.Log($"[MyceliaTest] {(passed ? "PASS" : "FAIL")}: {name}");
            return passed ? 0 : 1;
        }
        catch (System.Exception ex)
        {
            Debug.LogError($"[MyceliaTest] FAIL (exception): {name} — {ex.Message}");
            return 1;
        }
    }
}
