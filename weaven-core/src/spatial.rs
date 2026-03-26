/// Spatial Routing Layer (§7.1) — Tier 2.
///
/// Manages dynamic connections based on spatial relationships.
/// Uses a grid hash (configurable cell size) for O(1) approximate lookups.
///
/// Integration flow:
///   Phase 1: update_position() called when SMs move.
///   Phase 2: spatial conditions in InteractionRules evaluated using the index.
///   Phase 5: insert/remove when entities spawn/despawn.

use std::collections::{BTreeMap, HashMap};
use crate::types::SmId;

// ---------------------------------------------------------------------------
// Spatial Index (grid hash)
// ---------------------------------------------------------------------------

/// A spatial index supporting radius queries via grid hash bucketing.
///
/// Grid cell size determines trade-offs:
///   - Smaller cells → more precise, more memory, more hash lookups per query.
///   - Larger cells  → coarser, fewer cells, but more false positives per query.
///
/// Typical usage: set cell_size ≈ median influence radius.
#[derive(Debug)]
pub struct SpatialIndex {
    /// Cell size for grid bucketing.
    pub cell_size: f64,
    /// SM → (x, y) position.
    positions: BTreeMap<SmId, (f64, f64)>,
    /// Grid cell → set of SM IDs.
    /// Key: (cell_x, cell_y) as i64 integers.
    grid: HashMap<(i64, i64), Vec<SmId>>,
}

impl SpatialIndex {
    pub fn new(cell_size: f64) -> Self {
        assert!(cell_size > 0.0, "cell_size must be positive");
        Self {
            cell_size,
            positions: BTreeMap::new(),
            grid: HashMap::new(),
        }
    }

    /// Convert world coordinates to grid cell coordinates.
    fn to_cell(&self, x: f64, y: f64) -> (i64, i64) {
        ((x / self.cell_size).floor() as i64,
         (y / self.cell_size).floor() as i64)
    }

    /// Insert or update an SM's position.
    pub fn update(&mut self, sm_id: SmId, x: f64, y: f64) {
        // Remove from old cell if present.
        if let Some(&(old_x, old_y)) = self.positions.get(&sm_id) {
            let old_cell = self.to_cell(old_x, old_y);
            if let Some(v) = self.grid.get_mut(&old_cell) {
                v.retain(|&id| id != sm_id);
            }
        }
        // Insert into new cell.
        let cell = self.to_cell(x, y);
        self.positions.insert(sm_id, (x, y));
        self.grid.entry(cell).or_default().push(sm_id);
    }

    /// Remove an SM from the index (called on despawn).
    pub fn remove(&mut self, sm_id: SmId) {
        if let Some((x, y)) = self.positions.remove(&sm_id) {
            let cell = self.to_cell(x, y);
            if let Some(v) = self.grid.get_mut(&cell) {
                v.retain(|&id| id != sm_id);
            }
        }
    }

    /// Get the position of an SM, if registered.
    pub fn position(&self, sm_id: SmId) -> Option<(f64, f64)> {
        self.positions.get(&sm_id).copied()
    }

    /// Query all SM IDs within `radius` of (`x`, `y`).
    /// Returns candidates (may include false positives from adjacent cells).
    /// Callers should filter by exact distance if needed.
    pub fn query_radius(&self, x: f64, y: f64, radius: f64) -> Vec<SmId> {
        let cell_radius = (radius / self.cell_size).ceil() as i64;
        let center = self.to_cell(x, y);
        let r2 = radius * radius;

        let mut results = Vec::new();
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let cell = (center.0 + dx, center.1 + dy);
                if let Some(ids) = self.grid.get(&cell) {
                    for &sm_id in ids {
                        if let Some(&(sx, sy)) = self.positions.get(&sm_id) {
                            let dist2 = (sx - x) * (sx - x) + (sy - y) * (sy - y);
                            if dist2 <= r2 {
                                results.push(sm_id);
                            }
                        }
                    }
                }
            }
        }
        results
    }

    /// Query all SM IDs within `radius` of `sm_id`'s current position.
    /// Returns empty if sm_id is not in the index.
    pub fn query_radius_of(&self, sm_id: SmId, radius: f64) -> Vec<SmId> {
        match self.positions.get(&sm_id) {
            Some(&(x, y)) => self.query_radius(x, y, radius),
            None => vec![],
        }
    }

    /// Exact distance² between two SMs. Returns None if either is not in the index.
    pub fn distance_sq(&self, a: SmId, b: SmId) -> Option<f64> {
        let (ax, ay) = self.positions.get(&a).copied()?;
        let (bx, by) = self.positions.get(&b).copied()?;
        Some((ax - bx) * (ax - bx) + (ay - by) * (ay - by))
    }

    /// Exact distance between two SMs.
    pub fn distance(&self, a: SmId, b: SmId) -> Option<f64> {
        self.distance_sq(a, b).map(f64::sqrt)
    }

    pub fn sm_count(&self) -> usize { self.positions.len() }
    pub fn is_empty(&self) -> bool  { self.positions.is_empty() }
}

impl Default for SpatialIndex {
    fn default() -> Self { Self::new(10.0) }
}

// ---------------------------------------------------------------------------
// Spatial InteractionRule condition
// ---------------------------------------------------------------------------

/// A spatial condition that can be added to an InteractionRule.
/// Evaluated in Phase 2 against the SpatialIndex.
///
/// Example: "match only if sm_a and sm_b are within 5 units of each other."
pub type SpatialConditionFn =
    Box<dyn Fn(&SpatialIndex, SmId, SmId) -> bool + Send + Sync>;

/// Proximity condition: two SMs must be within `max_distance` of each other.
pub fn proximity(max_distance: f64) -> SpatialConditionFn {
    Box::new(move |spatial, a, b| {
        spatial.distance(a, b).map_or(false, |d| d <= max_distance)
    })
}

/// Any-proximity condition: SM `a` must be within `radius` of any SM in `targets`.
pub fn any_within_radius(radius: f64) -> SpatialConditionFn {
    Box::new(move |spatial, a, b| {
        spatial.distance(a, b).map_or(false, |d| d <= radius)
    })
}
