// w3_patch: fixed-capacity animated water patch / tile manager (water v4).
//
// Tracks at most 64 recently active water tiles on an integer X/Z grid.
// Each tile carries an "energy" value that decays every frame. When energy
// reaches zero the tile falls asleep and its capacity slot is recycled.
// Calling `ensure` wakes (or refreshes) a tile, so only tiles that are still
// being disturbed remain in the active slice.
//
// Mobile friendly design notes:
// - No HashMap, no Box, no Vec. All storage is fixed-size arrays allocated
//   inline with the manager (~2.8 KiB total), so there is zero allocation
//   churn in the hot path.
// - Grid keys are plain (i32, i32) pairs resolved through a fixed 128-slot
//   open-addressing table (linear probing + backward-shift deletion, so no
//   tombstones and no long-run table degradation).
// - Active patches stay contiguous at the front of the array, which makes
//   `active_iter()` a real contiguous slice with zero copying.

/// Hard capacity of the manager.
pub const MAX_PATCHES: usize = 64;
/// Power-of-two table size. Keeps load factor <= 0.5 for cheap probing.
const TABLE_SIZE: usize = 128;
const TABLE_MASK: usize = TABLE_SIZE - 1;
pub const DEFAULT_ENERGY_FULL: f32 = 1.0;
/// With the default rate an untouched tile sleeps after about 4 seconds.
pub const DEFAULT_DECAY_PER_SEC: f32 = 0.25;

/// One animated water tile on the X/Z grid.
#[derive(Clone, Copy, Debug, Default)]
pub struct WaterPatch {
    pub cx: i32,
    pub cz: i32,
    pub energy: f32,
    pub age: f32,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug)]
struct TableEntry {
    cx: i32,
    cz: i32,
    slot: u16,
    occupied: bool,
}

impl TableEntry {
    #[inline]
    fn empty() -> Self {
        Self { cx: 0, cz: 0, slot: 0, occupied: false }
    }
}

/// Mixed integer hash for an (i32, i32) grid key.
#[inline]
fn hash_key(cx: i32, cz: i32) -> usize {
    let mut h = (cx as u32).wrapping_mul(0x9E37_79B9) ^ (cz as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    (h as usize) & TABLE_MASK
}

/// Fixed-capacity animated water patch manager with energy sleep/wake.
#[derive(Clone, Debug)]
pub struct PatchManager {
    patches: [WaterPatch; MAX_PATCHES],
    table: [TableEntry; TABLE_SIZE],
    active_count: usize,
    /// Energy restored by `ensure` / `disturb`.
    pub energy_full: f32,
    /// Energy lost per second in `update`. Values <= 0 disable sleeping.
    pub decay_per_sec: f32,
}

impl Default for PatchManager {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl PatchManager {
    /// Creates an empty manager with default energy/decay settings.
    pub fn new() -> Self {
        Self {
            patches: [WaterPatch::default(); MAX_PATCHES],
            table: [TableEntry::empty(); TABLE_SIZE],
            active_count: 0,
            energy_full: DEFAULT_ENERGY_FULL,
            decay_per_sec: DEFAULT_DECAY_PER_SEC,
        }
    }

    /// Contiguous slice of awake tiles. Valid until the next `&mut self` call.
    #[inline]
    pub fn active_iter(&self) -> &[WaterPatch] {
        &self.patches[..self.active_count]
    }

    /// Alias for `active_iter` for callers that prefer an explicit name.
    #[inline]
    pub fn active_slice(&self) -> &[WaterPatch] {
        self.active_iter()
    }

    #[inline]
    pub fn active_len(&self) -> usize {
        self.active_count
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        MAX_PATCHES
    }

    #[inline]
    pub fn any_active(&self) -> bool {
        self.active_count != 0
    }

    #[inline]
    pub fn clear(&mut self) {
        for slot in 0..self.active_count {
            self.patches[slot] = WaterPatch::default();
        }
        self.table = [TableEntry::empty(); TABLE_SIZE];
        self.active_count = 0;
    }

    /// Looks up an awake tile by its grid cell.
    #[inline]
    pub fn get(&self, cx: i32, cz: i32) -> Option<&WaterPatch> {
        self.find_key(cx, cz).map(|slot| &self.patches[slot])
    }

    #[inline]
    pub fn contains(&self, cx: i32, cz: i32) -> bool {
        self.find_key(cx, cz).is_some()
    }

    /// Ensures a tile exists at (cx, cz), waking or refreshing it.
    ///
    /// Returns the patch slot index. If the manager is full, the awake tile
    /// with the least energy is recycled to make room for the new cell.
    pub fn ensure(&mut self, cx: i32, cz: i32) -> usize {
        if let Some(slot) = self.find_key(cx, cz) {
            let patch = &mut self.patches[slot];
            patch.energy = self.energy_full;
            patch.alive = true;
            return slot;
        }

        if self.active_count == MAX_PATCHES {
            let victim = self.lowest_energy_index();
            self.deactivate_at(victim);
        }

        let slot = self.active_count;
        let patch = &mut self.patches[slot];
        patch.cx = cx;
        patch.cz = cz;
        patch.energy = self.energy_full;
        patch.age = 0.0;
        patch.alive = true;
        self.active_count += 1;
        self.insert_key(cx, cz, slot);
        slot
    }

    /// Sets a tile's energy to `amount` (clamped), creating it if needed.
    /// Useful for wakes that should sleep sooner than a full `ensure` refresh.
    pub fn disturb(&mut self, cx: i32, cz: i32, amount: f32) -> usize {
        let slot = self.ensure(cx, cz);
        self.patches[slot].energy = amount.clamp(0.0, self.energy_full);
        slot
    }

    /// Directly sets a tile's energy (clamped), without waking a sleeping one.
    /// Returns true when the tile was awake and found.
    pub fn set_energy(&mut self, cx: i32, cz: i32, energy: f32) -> bool {
        match self.find_key(cx, cz) {
            Some(slot) => {
                self.patches[slot].energy = energy.clamp(0.0, self.energy_full);
                true
            }
            None => false,
        }
    }

    /// Advances all awake tiles by `dt` seconds, putting exhausted ones to sleep.
    pub fn update(&mut self, dt: f32) {
        if dt <= 0.0 || self.active_count == 0 {
            return;
        }
        let decay = if self.decay_per_sec > 0.0 {
            self.decay_per_sec * dt
        } else {
            0.0
        };
        let mut i = 0;
        while i < self.active_count {
            let sleep = {
                let patch = &mut self.patches[i];
                patch.age += dt;
                patch.energy -= decay;
                patch.energy <= 0.0 || !patch.alive
            };
            if sleep {
                // A tile from the end is swapped into slot i; process it too.
                self.deactivate_at(i);
            } else {
                i += 1;
            }
        }
    }

    /// Removes the awake tile at `slot`, keeping the active prefix dense.
    fn deactivate_at(&mut self, slot: usize) {
        let last = self.active_count - 1;
        let (cx, cz) = (self.patches[slot].cx, self.patches[slot].cz);
        self.remove_key(cx, cz);
        self.patches[slot].alive = false;
        self.patches[slot].energy = 0.0;
        if slot != last {
            self.patches.swap(slot, last);
            let moved = self.patches[slot];
            self.set_slot(moved.cx, moved.cz, slot);
        }
        self.active_count = last;
    }

    #[inline]
    fn lowest_energy_index(&self) -> usize {
        let mut best = 0;
        let mut best_energy = self.patches[0].energy;
        for i in 1..self.active_count {
            let e = self.patches[i].energy;
            if e < best_energy {
                best_energy = e;
                best = i;
            }
        }
        best
    }

    #[inline]
    fn find_key(&self, cx: i32, cz: i32) -> Option<usize> {
        let mut i = hash_key(cx, cz);
        loop {
            let entry = self.table[i];
            if !entry.occupied {
                return None;
            }
            if entry.cx == cx && entry.cz == cz {
                return Some(entry.slot as usize);
            }
            i = (i + 1) & TABLE_MASK;
        }
    }

    #[inline]
    fn insert_key(&mut self, cx: i32, cz: i32, slot: usize) {
        let mut i = hash_key(cx, cz);
        loop {
            if !self.table[i].occupied {
                self.table[i] = TableEntry {
                    cx,
                    cz,
                    slot: slot as u16,
                    occupied: true,
                };
                return;
            }
            i = (i + 1) & TABLE_MASK;
        }
    }

    #[inline]
    fn set_slot(&mut self, cx: i32, cz: i32, slot: usize) {
        let mut i = hash_key(cx, cz);
        loop {
            let entry = self.table[i];
            debug_assert!(entry.occupied, "set_slot called for missing key");
            if entry.cx == cx && entry.cz == cz {
                self.table[i].slot = slot as u16;
                return;
            }
            i = (i + 1) & TABLE_MASK;
        }
    }

    /// Backward-shift deletion: no tombstones, table stays clean indefinitely.
    fn remove_key(&mut self, cx: i32, cz: i32) {
        let mut i = hash_key(cx, cz);
        loop {
            let entry = self.table[i];
            if !entry.occupied {
                return;
            }
            if entry.cx == cx && entry.cz == cz {
                break;
            }
            i = (i + 1) & TABLE_MASK;
        }

        let mut hole = i;
        loop {
            self.table[hole] = TableEntry::empty();
            let mut j = hole;
            loop {
                j = (j + 1) & TABLE_MASK;
                if !self.table[j].occupied {
                    return;
                }
                let entry = self.table[j];
                let home = hash_key(entry.cx, entry.cz);
                // Move entry j back into the hole if its home is not in the
                // cyclic interval (hole, j]; otherwise it must stay at j.
                if j.wrapping_sub(home) & TABLE_MASK >= j.wrapping_sub(hole) & TABLE_MASK {
                    self.table[hole] = entry;
                    hole = j;
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_wakes_refreshes_and_deduplicates_tiles() {
        let mut m = PatchManager::new();
        let a = m.ensure(3, -7);
        let b = m.ensure(-2, 9);
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(m.active_len(), 2);
        assert_eq!(m.active_iter().len(), 2);
        assert_eq!(m.active_iter()[0].cx, 3);
        assert_eq!(m.active_iter()[1].cz, 9);
        assert!(m.contains(3, -7) && m.contains(-2, 9));

        assert!(m.set_energy(3, -7, 0.2));
        let again = m.ensure(3, -7);
        assert_eq!(again, a);
        assert_eq!(m.active_len(), 2);
        assert_eq!(m.get(3, -7).unwrap().energy, m.energy_full);
        assert!(m.get(3, -7).unwrap().alive);
    }

    #[test]
    fn energy_decay_sleeps_then_ensure_rewakes() {
        let mut m = PatchManager::new();
        m.ensure(0, 0);
        m.update(1.0);
        assert_eq!(m.active_len(), 1);
        assert_eq!(m.get(0, 0).unwrap().age, 1.0);
        assert_eq!(m.get(0, 0).unwrap().energy, 0.75);

        m.update(3.0); // total decay 1.0 -> exact sleep
        assert_eq!(m.active_len(), 0);
        assert!(!m.any_active());
        assert!(!m.contains(0, 0));
        assert!(!m.patches[0].alive);
        assert!(m.active_iter().is_empty());

        let slot = m.ensure(0, 0);
        assert_eq!(slot, 0);
        assert_eq!(m.active_len(), 1);
        let p = m.get(0, 0).unwrap();
        assert_eq!(p.energy, m.energy_full);
        assert_eq!(p.age, 0.0);
        assert!(p.alive);
    }

    #[test]
    fn capacity_evicts_lowest_energy_tile() {
        let mut m = PatchManager::new();
        for i in 0..MAX_PATCHES {
            m.ensure(i as i32, 0);
        }
        assert_eq!(m.active_len(), MAX_PATCHES);

        m.set_energy(0, 0, 0.05);
        m.set_energy(1, 0, 0.20);
        m.ensure(999, -999);

        assert_eq!(m.active_len(), MAX_PATCHES);
        assert!(m.contains(999, -999));
        assert!(!m.contains(0, 0));
        assert!(m.contains(1, 0));
        for p in m.active_iter() {
            assert!(p.alive && p.energy > 0.0);
        }
    }

    #[test]
    fn backward_shift_deletion_keeps_lookup_consistent() {
        let mut m = PatchManager::new();
        let keys: [(i32, i32); 12] = [
            (0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5),
            (6, 6), (7, 7), (8, 8), (-9, 2), (1000, -1000), (4, -3),
        ];
        for &(x, z) in &keys {
            m.ensure(x, z);
        }

        for &(x, z) in &[(2, 2), (5, 5), (8, 8)] {
            assert!(m.set_energy(x, z, 0.01));
        }
        m.update(0.1);
        assert_eq!(m.active_len(), 9);

        for &(x, z) in &keys {
            let should_live = !matches!((x, z), (2, 2) | (5, 5) | (8, 8));
            assert_eq!(m.contains(x, z), should_live, "key ({x},{z})");
        }

        for &(x, z) in &keys {
            m.ensure(x, z);
        }
        assert_eq!(m.active_len(), 12);
        for &(x, z) in &keys {
            assert!(m.contains(x, z));
        }
    }

    #[test]
    fn disturb_sets_partial_energy() {
        let mut m = PatchManager::new();
        let slot = m.disturb(7, 7, 0.3);
        assert_eq!(slot, 0);
        assert_eq!(m.get(7, 7).unwrap().energy, 0.3);
        assert_eq!(m.active_len(), 1);

        m.disturb(7, 7, 5.0);
        assert_eq!(m.get(7, 7).unwrap().energy, m.energy_full);
    }

    #[test]
    fn non_positive_dt_and_clear_are_noops_or_reset() {
        let mut m = PatchManager::new();
        m.ensure(5, 5);
        m.update(-1.0);
        m.update(0.0);
        assert_eq!(m.active_len(), 1);
        assert_eq!(m.get(5, 5).unwrap().energy, m.energy_full);
        assert_eq!(m.get(5, 5).unwrap().age, 0.0);

        m.clear();
        assert_eq!(m.active_len(), 0);
        assert!(!m.contains(5, 5));
        m.ensure(5, 5);
        assert!(m.contains(5, 5));
    }
}
