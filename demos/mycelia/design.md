# Mycelia - 3D Sandbox RPG Demo Design

## Concept

> Mycelia is a 3D sandbox RPG where you play as a "Forest Guardian" who
> rebuilds ecosystems by cultivating fungal mycelium networks.
> The underground mycelium acts as a nervous system connecting all
> life in the forest, transmitting nutrients and warning signals.

## Demo Scope (Phase 1 - Minimum Playable)

### Entities

| Entity | SM Type | Instances | Description |
|--------|---------|-----------|-------------|
| Grass | Plant | many | Fast-growing, low requirements |
| Tree | Plant | few | Slow, provides shade, deep roots |
| Moonlight Orchid | Plant | 1 (goal) | Requires specific conditions to bloom |
| Nutrient Fungus | Fungus | many | Transfers nutrients between connected plants |
| Moth | Creature | few | Pollinates flowering plants at night |
| Caterpillar | Creature | few | Pest, eats leaves, damages plants |
| Soil Cell | Environment | grid | Tracks moisture, pH, nutrients, shade |

### Win Condition

Bloom the Moonlight Orchid. Requires:
1. Shade (tree canopy above)
2. Acidic soil (pH 5.5-6.0, from conifer leaf litter)
3. Mycorrhizal connection (nutrient fungus linked)
4. Pollinator present (moth)
5. Sufficient moisture

---

## State Machine Definitions

### SM 1: Plant (generic, parameterized via context)

```
States: seed(0) -> sprout(1) -> growing(2) -> mature(3) -> flowering(4) -> wilting(5) -> dead(6)
```

Context fields (per instance):
- `species`: string (grass/tree/orchid)
- `hp`: float (0-100)
- `growth`: float (0-100, progress to next stage)
- `moisture_need`: float (from Named Table)
- `light_need`: float (from Named Table)
- `ph_min`, `ph_max`: float (acceptable pH range)
- `has_pollinator`: bool
- `has_mycorrhiza`: bool

Transitions:
- `seed -> sprout`: guard(moisture > moisture_need AND light > light_need)
- `sprout -> growing`: guard(growth > 30)
- `growing -> mature`: guard(growth > 70)
- `mature -> flowering`: guard(growth > 90 AND has_mycorrhiza AND species-specific conditions)
- `flowering -> wilting`: guard(hp < 20 OR moisture == 0)  
- `any -> wilting`: guard(hp <= 0)
- `wilting -> dead`: guard(hp <= 0 AND growth tick)

Effects on transitions:
- `mature -> flowering` emits Signal{port: "pollen_ready"} (attracts pollinators)
- `wilting -> dead` emits Signal{port: "decompose"} (triggers decomposer fungi)

Growth tick: each tick, if conditions met, growth += growth_rate. If not, hp decreases.

### SM 2: Fungus

```
States: spore(0) -> germinating(1) -> growing(2) -> connected(3)
```

Context fields:
- `network_size`: int (number of connected plants)
- `nutrient_pool`: float
- `signal_range`: float

Transitions:
- `spore -> germinating`: guard(soil moisture > 40)
- `germinating -> growing`: guard(growth > 50)
- `growing -> connected`: guard(nearby_plant AND growth > 80)

Effects:
- `connected` state: each tick, transfer nutrients to connected plants (Signal to plant input port)
- On receiving "warning" signal: propagate to all connected plants (cascade)

### SM 3: Creature

```
States: idle(0) -> moving(1) -> acting(2) -> fleeing(3)
```

Context fields:
- `species`: string (moth/caterpillar)
- `hunger`: float (0-100)
- `target_sm`: int (current target entity)

Transitions:
- Moth: `idle -> moving`: guard(nearby flowering plant)
- Moth: `moving -> acting`: guard(at target, pollinating)
- Caterpillar: `idle -> moving`: guard(hunger > 50 AND nearby plant)
- Caterpillar: `moving -> acting`: guard(at target, eating)
- Any: `* -> fleeing`: guard(threat detected)

### SM 4: Soil Cell

```
States: barren(0) -> poor(1) -> moderate(2) -> rich(3)
```

Context fields:
- `moisture`: float (0-100)
- `ph`: float (3.0-9.0)
- `nutrients`: float (0-100)
- `shade_level`: float (0-1, from tree canopy)
- `organic_matter`: float (0-100)

Transitions:
- `barren -> poor`: guard(organic_matter > 20)
- `poor -> moderate`: guard(organic_matter > 50 AND nutrients > 30)
- `moderate -> rich`: guard(organic_matter > 80 AND nutrients > 60 AND moisture > 40)
- Reverse transitions when conditions degrade

---

## Interaction Rules

### IR 1: Mycorrhizal Nutrient Transfer
- Participants: Fungus(connected) + Plant(any growing state)
- Spatial: radius 10 (mycelium reach)
- Guard: fungus.nutrient_pool > 5
- Effect: Signal to plant { nutrients: 10 }, SetContext fungus.nutrient_pool -= 10

### IR 2: Pest Damage
- Participants: Caterpillar(acting) + Plant(growing/mature/flowering)
- Spatial: radius 2 (contact)
- Guard: caterpillar.hunger > 30
- Effect: Signal to plant { damage: 15 }, SetContext caterpillar.hunger -= 30

### IR 3: Pollination
- Participants: Moth(acting) + Plant(flowering)
- Spatial: radius 3
- Guard: plant.species requires pollination
- Effect: SetContext plant.has_pollinator = true

### IR 4: Warning Signal Cascade
- Participants: Plant(wilting) + Fungus(connected)
- Spatial: radius 10
- Guard: plant.hp < 30
- Effect: Signal via fungus network to all connected plants { warning: true }
  (connected plants boost defense, reducing pest damage)

### IR 5: Shade Effect
- Participants: Tree(mature/flowering) + SoilCell(any)
- Spatial: radius 5 (canopy)
- Guard: tree.growth > 70
- Effect: SetContext soil.shade_level = 0.7

### IR 6: Decomposition
- Participants: Fungus(connected) + Plant(dead)
- Spatial: radius 10
- Guard: true
- Effect: SetContext soil.organic_matter += 20, soil.nutrients += 10

---

## Named Tables

### plant_species
```json
{
  "grass":   { "growth_rate": 5.0, "moisture_need": 20, "light_need": 0.5, "ph_min": 5.0, "ph_max": 8.0, "max_hp": 50 },
  "tree":    { "growth_rate": 1.0, "moisture_need": 40, "light_need": 0.7, "ph_min": 4.5, "ph_max": 7.0, "max_hp": 200 },
  "orchid":  { "growth_rate": 0.5, "moisture_need": 60, "light_need": 0.3, "ph_min": 5.5, "ph_max": 6.0, "max_hp": 30 }
}
```

### fungus_compatibility
```json
{
  "nutrient": {
    "grass": { "transfer_rate": 1.0, "compatible": true },
    "tree":  { "transfer_rate": 2.0, "compatible": true },
    "orchid": { "transfer_rate": 3.0, "compatible": true }
  }
}
```

---

## Connections

### Plant -> Fungus Network
- Plant output port "decompose" -> Fungus input port "organic_input"
  - Pipeline: Transform { organic_value: growth * 0.5 }

### Fungus -> Plant Nutrient Feed
- Fungus output port "nutrient_out" -> Plant input port "nutrient_in"
  - Pipeline: Filter { nutrients > 0 }

### Fungus -> Fungus Warning Relay
- Fungus output port "warning_out" -> Fungus input port "warning_in"
  - Pipeline: Transform { intensity: intensity - 1 }, Filter { intensity > 0 }
  - (Signal attenuates as it travels through the network)

---

## Testing Strategy

### Layer 1: Weaven Core (cargo test)
- Plant SM lifecycle: seed -> sprout -> ... -> flowering with correct conditions
- Fungus connection and nutrient transfer
- Pest damage reduces plant HP
- Warning signal cascade through fungus network
- Moonlight orchid bloom conditions (all 5 requirements)
- Named table lookups for species parameters

### Layer 2: Unity C# Adapter (dotnet test)
- WeavenWorld integration with plant/fungus/creature SMs
- Tick simulation: N ticks with varying environment conditions
- Win condition detection

### Layer 3: Unity Headless (batchmode)
- Game scene loads without errors
- 100-tick simulation runs without crash
- Save/load state via snapshot

### Layer 4: Visual Verification (xvfb + screenshot)
- Plants visually change appearance per state
- Mycelium network rendered as glowing lines underground
- UI shows resource counts and win progress
