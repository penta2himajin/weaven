using System.Collections.Generic;
using System.IO;
using UnityEngine;
using Weaven;

/// <summary>
/// Core Mycelia simulation driver.
/// Manages the Weaven world, entities, and tick loop for the ecosystem RPG.
///
/// Architecture:
///   - Each plant/fungus/creature/soil is a Weaven SM instance
///   - Spatial positions are synchronized from Unity transforms to Weaven
///   - Weaven ticks drive state transitions; Unity renders the results
///   - Interaction Rules handle cross-entity mechanics (nutrient transfer,
///     pest damage, pollination, warning cascades, decomposition)
/// </summary>
public class MyceliaSimulation : MonoBehaviour
{
    [Header("Schema")]
    [SerializeField] private TextAsset schemaJson;

    [Header("Simulation")]
    [SerializeField] private float tickInterval = 0.5f;

    private WeavenWorld _weaven;
    private float _tickTimer;
    private Dictionary<uint, GameObject> _entityObjects = new();
    private uint _nextEntityId = 10; // SM IDs 1-4 are templates; instances start at 10

    // Entity tracking
    private List<uint> _plants = new();
    private List<uint> _fungi = new();
    private List<uint> _creatures = new();
    private List<uint> _soilCells = new();

    // Win condition
    public bool OrchidBloomed { get; private set; }

    void Awake()
    {
        _weaven = new WeavenWorld();

        string json = schemaJson != null
            ? schemaJson.text
            : File.ReadAllText(Path.Combine(Application.dataPath, "..", "..", "mycelia.json"));

        _weaven.LoadSchema(json);
        _weaven.EnableSpatial(10.0);

        Debug.Log("[Mycelia] Simulation initialized");
    }

    void FixedUpdate()
    {
        _tickTimer += Time.fixedDeltaTime;
        if (_tickTimer < tickInterval) return;
        _tickTimer -= tickInterval;

        SyncPositionsToWeaven();
        UpdateEnvironment();
        SimulateTick();
    }

    void OnDestroy()
    {
        _weaven?.Dispose();
    }

    // ── Public API for game systems ─────────────────────────────────────

    /// <summary>
    /// Plant a seed at the given position with species-specific parameters.
    /// Species: 0=grass, 1=tree, 2=orchid
    /// </summary>
    public uint PlantSeed(Vector3 position, int species)
    {
        uint id = _nextEntityId++;
        // In a full implementation, this would spawn a new SM instance.
        // For now, we use the template SM IDs for the first instances.
        _plants.Add(id);

        // Set species parameters from Named Table (done here because
        // context is f64-only, and species names are strings)
        double moistureNeed = species switch { 0 => 20, 1 => 40, 2 => 60, _ => 20 };
        double lightNeed = species switch { 0 => 0.5, 1 => 0.7, 2 => 0.3, _ => 0.5 };
        double maxHp = species switch { 0 => 50, 1 => 200, 2 => 30, _ => 50 };
        double growthRate = species switch { 0 => 5, 1 => 1, 2 => 0.5, _ => 5 };
        double needsMycorrhiza = species == 2 ? 1.0 : 0.0;
        double needsPollinator = species == 2 ? 1.0 : 0.0;

        _weaven.PushInput(1, "moisture_need", moistureNeed);
        _weaven.PushInput(1, "light_need", lightNeed);
        _weaven.PushInput(1, "hp", maxHp);
        _weaven.PushInput(1, "growth_rate", growthRate);
        _weaven.PushInput(1, "needs_mycorrhiza", needsMycorrhiza);
        _weaven.PushInput(1, "needs_pollinator", needsPollinator);

        Debug.Log($"[Mycelia] Planted species {species} at {position}");
        return id;
    }

    /// <summary>
    /// Place a fungal spore at the given position.
    /// </summary>
    public uint PlaceFungus(Vector3 position)
    {
        uint id = _nextEntityId++;
        _fungi.Add(id);
        _weaven.PushInput(2, "nutrient_pool", 20.0);
        Debug.Log($"[Mycelia] Placed fungus at {position}");
        return id;
    }

    /// <summary>
    /// Get the current state name for a plant SM.
    /// </summary>
    public string GetPlantStateName(uint smId)
    {
        int? state = _weaven.ActiveState(smId);
        return state switch
        {
            0 => "seed",
            1 => "sprout",
            2 => "growing",
            3 => "mature",
            4 => "flowering",
            5 => "wilting",
            6 => "dead",
            _ => "unknown"
        };
    }

    // ── Internal simulation ─────────────────────────────────────────────

    private void SyncPositionsToWeaven()
    {
        foreach (var (smId, go) in _entityObjects)
        {
            var pos = go.transform.position;
            _weaven.SetPosition(smId, pos.x, pos.z);
        }
    }

    private void UpdateEnvironment()
    {
        // Simulate environment: update moisture/light based on weather, time of day
        // For demo: simple sine-wave day/night cycle
        double timeOfDay = (Time.time % 60.0) / 60.0; // 0-1 cycle
        double lightLevel = Mathf.Max(0, Mathf.Sin((float)(timeOfDay * Mathf.PI)));

        _weaven.PushInput(1, "light", lightLevel);

        // Moisture from rain (periodic)
        double moisture = 30 + 20 * Mathf.Sin((float)(Time.time * 0.1));
        _weaven.PushInput(1, "moisture", moisture);
        _weaven.PushInput(2, "moisture", moisture);
    }

    private void SimulateTick()
    {
        // Activate all SMs
        foreach (var id in _weaven.SmIds)
        {
            _weaven.Activate(id);
        }

        // Increment growth for plants
        double currentGrowth = _weaven.ReadOutput(1, "growth");
        double growthRate = _weaven.ReadOutput(1, "growth_rate");
        _weaven.PushInput(1, "growth", currentGrowth + growthRate);

        // Increment growth for fungi
        double fungusGrowth = _weaven.ReadOutput(2, "growth");
        _weaven.PushInput(2, "growth", fungusGrowth + 2.0);

        var result = _weaven.Tick();

        // Process state changes
        foreach (var (smId, change) in result.StateChanges)
        {
            Debug.Log($"[Mycelia] SM {smId}: state {change.Prev} -> {change.Next}");

            // Check orchid bloom win condition
            if (smId == 1 && change.Next == 4)
            {
                int? needsMycorrhiza = null;
                // If this was the orchid, we won!
                OrchidBloomed = true;
                Debug.Log("[Mycelia] *** MOONLIGHT ORCHID BLOOMED — YOU WIN! ***");
            }
        }

        // Process system commands
        foreach (var cmd in result.SystemCommands)
        {
            Debug.Log($"[Mycelia] System command: {cmd}");
        }
    }
}
