use std::collections::HashSet;

use crate::curse::TICK_MS;
use crate::naming::{name_item, ItemName};
use crate::piece::{
    default_cooldown_ms, EffectKind, PieceId, PieceKind, PieceRegistry, SlotKind, Trigger,
};
use crate::slot::{PlaceError, Slot};
use crate::stats::{StatKind, Stats};

/// One assembled item, reduced to what combat needs: how often it fires and
/// what happens when it does.
#[derive(Clone, Debug)]
pub struct ItemProfile {
    /// The item's generated short name — what the cooldown bars show.
    pub name: String,
    /// The same name with its "of the ..." tail.
    pub full_name: String,
    /// The core component it was built around, for reference.
    pub core: String,
    pub slot: SlotKind,
    pub cooldown_ms: u32,
    pub stats: Stats,
    pub triggers: Vec<Trigger>,
    /// Assembled items in the same slot touching this one, counted once.
    pub adjacent_assembled_same_slot: usize,
}

/// Name an item by its core piece, falling back to the first piece it has.
fn core_name(reg: &PieceRegistry, pieces: &[PieceId]) -> String {
    pieces
        .iter()
        .copied()
        .find(|&p| reg.def(p).kind.is_core())
        .or_else(|| pieces.first().copied())
        .map(|p| reg.def(p).name.to_string())
        .unwrap_or_default()
}

/// One orthogonally-connected group of components inside a slot — a candidate
/// piece of gear. A slot can hold as many of these as the player can fit
/// without them touching.
#[derive(Clone, Debug)]
pub struct GearItem {
    pub pieces: Vec<PieceId>,
    /// Procedurally generated from the run seed and this exact arrangement.
    pub name: ItemName,
    pub assembled: bool,
    /// "assembled" when it came together, otherwise what it is missing.
    pub status: String,
    /// Everything this item contributes, effects included.
    pub stats: Stats,
    /// Human-readable notes on every bonus and effect that actually fired.
    pub notes: Vec<String>,
}

/// The verdict on one slot: the items in it, and what they add up to.
#[derive(Clone, Debug)]
pub struct SlotReport {
    pub slot: SlotKind,
    pub items: Vec<GearItem>,
    pub stats: Stats,
}

impl SlotReport {
    pub fn assembled_count(&self) -> usize {
        self.items.iter().filter(|i| i.assembled).count()
    }

    pub fn loose_count(&self) -> usize {
        self.items.iter().filter(|i| !i.assembled).count()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Every note from every item, flattened.
    pub fn notes(&self) -> Vec<String> {
        self.items.iter().flat_map(|i| i.notes.clone()).collect()
    }

    /// One line for the UI: how many finished items, how many loose groups.
    pub fn summary(&self) -> String {
        if self.items.is_empty() {
            return "empty".to_string();
        }
        let done = self.assembled_count();
        let loose = self.loose_count();
        match (done, loose) {
            (0, _) => self
                .items
                .first()
                .map(|i| i.status.clone())
                .unwrap_or_else(|| "incomplete".to_string()),
            (n, 0) if n == 1 => "1 item assembled".to_string(),
            (n, 0) => format!("{} items assembled", n),
            (n, l) => format!("{} assembled, {} loose", n, l),
        }
    }
}

/// The character's five equipment grids.
#[derive(Clone, Debug)]
pub struct Loadout {
    pub slots: Vec<Slot>,
    /// Seeds the item-name generator. Set from the run's seed so a given run
    /// names a given arrangement consistently.
    pub name_seed: u64,
}

impl Default for Loadout {
    fn default() -> Self {
        Self::new()
    }
}

impl Loadout {
    pub fn new() -> Self {
        Loadout {
            slots: SlotKind::ALL.iter().map(|&k| Slot::new(k)).collect(),
            name_seed: 0,
        }
    }

    pub fn slot(&self, kind: SlotKind) -> &Slot {
        &self.slots[kind.index()]
    }

    pub fn slot_mut(&mut self, kind: SlotKind) -> &mut Slot {
        &mut self.slots[kind.index()]
    }

    /// Which slot, if any, currently holds `id`.
    pub fn slot_holding(&self, id: PieceId) -> Option<SlotKind> {
        self.slots.iter().find(|s| s.contains(id)).map(|s| s.kind)
    }

    /// Remove `id` from whichever slot holds it.
    pub fn remove_anywhere(&mut self, id: PieceId) {
        for s in &mut self.slots {
            s.remove(id);
        }
    }

    pub fn can_place(
        &self,
        reg: &PieceRegistry,
        id: PieceId,
        kind: SlotKind,
        ax: u8,
        ay: u8,
    ) -> Result<(), PlaceError> {
        self.slot(kind).can_place(reg, id, ax, ay)
    }

    pub fn reports(&self, reg: &PieceRegistry) -> Vec<SlotReport> {
        SlotKind::ALL.iter().map(|&k| self.report(reg, k)).collect()
    }

    /// Evaluate one slot.
    ///
    /// Ordering matters, because effects can be conditional on assembly:
    ///   1. split the slot into items (one per core piece)
    ///   2. decide which items satisfy the recipe (nothing has contributed
    ///      stats yet, so this can't depend on effect results)
    ///   3. total each item's stats, applying within-item effects
    ///   4. apply cross-item effects, which need every item's step-3 total
    ///   5. add the flat assembly bonuses of assembled items
    pub fn report(&self, reg: &PieceRegistry, kind: SlotKind) -> SlotReport {
        let slot = self.slot(kind);
        let groups = slot.items(reg);

        // 2.
        let verdicts: Vec<Result<(), String>> =
            groups.iter().map(|g| check_recipe(kind, reg, g)).collect();
        let assembled: Vec<bool> = verdicts.iter().map(|v| v.is_ok()).collect();

        let group_index_of = |id: PieceId| -> Option<usize> {
            groups.iter().position(|g| g.contains(&id))
        };
        let assembled_of =
            |id: PieceId| -> bool { group_index_of(id).map(|i| assembled[i]).unwrap_or(false) };

        // 3.
        let mut stats: Vec<Stats> = Vec::with_capacity(groups.len());
        let mut notes: Vec<Vec<String>> = Vec::with_capacity(groups.len());

        for (gi, group) in groups.iter().enumerate() {
            let mut item_stats = Stats::ZERO;
            let mut item_notes: Vec<String> = Vec::new();

            for &p in group {
                let def = reg.def(p);
                let mut contribution = def.base;

                if let Some(eff) = def.effect {
                    if let EffectKind::SelfPerEmptyCell { stat, per } = eff.kind {
                        if eff.when.holds(assembled[gi]) {
                            let n = slot.empty_neighbor_cells(p) as i32;
                            if n > 0 {
                                contribution.add(stat, per * n);
                                item_notes.push(format!(
                                    "{}: +{} {} from {} empty cells",
                                    def.name,
                                    per * n,
                                    stat.name(),
                                    n
                                ));
                            }
                        }
                    }
                }

                let mut doubled: HashSet<StatKind> = HashSet::new();
                for q in slot.neighbors_of(p) {
                    let Some(eff) = reg.def(q).effect else { continue };
                    let EffectKind::DoubleNeighbor { kind: target, stat } = eff.kind else {
                        continue;
                    };
                    if target == def.kind && eff.when.holds(assembled_of(q)) {
                        doubled.insert(stat);
                    }
                }
                for stat in doubled {
                    let before = contribution.get(stat);
                    if before != 0 {
                        contribution.set(stat, before * 2);
                        item_notes.push(format!(
                            "{}: {} doubled to {}",
                            def.name,
                            stat.name(),
                            before * 2
                        ));
                    }
                }

                item_stats += contribution;
            }
            stats.push(item_stats);
            notes.push(item_notes);
        }

        // 4. Cross-item: a piece can double a stat on every OTHER assembled
        //    item touching it. Reads the step-3 totals and writes new ones, so
        //    two such pieces can never feed each other in a loop.
        let snapshot = stats.clone();
        for (gi, group) in groups.iter().enumerate() {
            for &p in group {
                let Some(eff) = reg.def(p).effect else { continue };
                let EffectKind::DoubleAdjacentItemStat { stat } = eff.kind else { continue };
                if !eff.when.holds(assembled[gi]) {
                    continue;
                }
                for (gj, other) in groups.iter().enumerate() {
                    if gj == gi || !assembled[gj] {
                        continue;
                    }
                    if !slot.sets_touch(&[p], other) {
                        continue;
                    }
                    let before = snapshot[gj].get(stat);
                    if before != 0 {
                        stats[gj].add(stat, before);
                        notes[gj].push(format!(
                            "{}: {} doubled to {} by {}",
                            core_name(reg, other),
                            stat.name(),
                            before * 2,
                            reg.def(p).name
                        ));
                    }
                }
            }
        }

        // 5.
        let mut items = Vec::new();
        let mut slot_total = Stats::ZERO;
        for (gi, group) in groups.iter().enumerate() {
            let mut item_stats = stats[gi];
            let mut item_notes = std::mem::take(&mut notes[gi]);
            if assembled[gi] {
                for &p in group {
                    if let Some(adj) = reg.def(p).adjacency {
                        item_stats += adj.stats;
                        item_notes.push(adj.label.to_string());
                    }
                }
            }
            slot_total += item_stats;
            items.push(GearItem {
                name: name_item(self.name_seed, reg, slot, group),
                pieces: group.clone(),
                assembled: assembled[gi],
                status: match &verdicts[gi] {
                    Ok(()) => "assembled".to_string(),
                    Err(reason) => reason.clone(),
                },
                stats: item_stats,
                notes: item_notes,
            });
        }

        SlotReport { slot: kind, items, stats: slot_total }
    }

    /// Activation profiles for every assembled item across every slot — what
    /// combat actually runs on.
    pub fn combat_items(&self, reg: &PieceRegistry) -> Vec<ItemProfile> {
        let mut out = Vec::new();
        for kind in SlotKind::ALL {
            let slot = self.slot(kind);
            let report = self.report(reg, kind);
            let groups: Vec<&Vec<PieceId>> = report.items.iter().map(|i| &i.pieces).collect();

            for (gi, item) in report.items.iter().enumerate() {
                if !item.assembled {
                    continue;
                }
                // How many other finished items touch this one.
                let touching_same_slot = groups
                    .iter()
                    .enumerate()
                    .filter(|(gj, other)| {
                        *gj != gi && report.items[*gj].assembled && slot.sets_touch(item.pieces.as_slice(), other)
                    })
                    .count();

                let core = item
                    .pieces
                    .iter()
                    .copied()
                    .find(|&p| reg.def(p).kind.is_core());

                let base_cd = core
                    .map(|c| {
                        let d = reg.def(c).cooldown_ms;
                        if d == 0 { default_cooldown_ms(kind) } else { d }
                    })
                    .unwrap_or_else(|| default_cooldown_ms(kind));

                let speed: i32 =
                    100 + item.pieces.iter().map(|&p| reg.def(p).speed_bonus).sum::<i32>();
                let speed = speed.max(10);
                let cooldown_ms = ((base_cd as i64 * 100 / speed as i64) as u32).max(TICK_MS);

                let triggers: Vec<Trigger> = item
                    .pieces
                    .iter()
                    .flat_map(|&p| reg.def(p).triggers.iter().copied())
                    .collect();

                out.push(ItemProfile {
                    name: item.name.short.clone(),
                    full_name: item.name.full.clone(),
                    core: core.map(|c| reg.def(c).name.to_string()).unwrap_or_default(),
                    slot: kind,
                    cooldown_ms,
                    stats: item.stats,
                    triggers,
                    adjacent_assembled_same_slot: touching_same_slot,
                });
            }
        }
        out
    }

    /// Base character stats plus every slot's contribution.
    pub fn total_stats(&self, reg: &PieceRegistry) -> Stats {
        let mut total = Stats::base_character();
        for r in self.reports(reg) {
            total += r.stats;
        }
        total
    }
}

/// Does this group of components satisfy the slot's recipe? Returns the
/// missing-requirement message on failure, phrased for the player. Counts are
/// per item, not per slot — two complete weapons in one slot is legal.
fn check_recipe(kind: SlotKind, reg: &PieceRegistry, pieces: &[PieceId]) -> Result<(), String> {
    let counts = Slot::kind_counts(reg, pieces);
    let n = |k: PieceKind| counts.get(&k).copied().unwrap_or(0);

    let reqs: &[(PieceKind, usize, usize)] = match kind {
        SlotKind::Weapon => &[
            (PieceKind::Handle, 1, 1),
            (PieceKind::Damaging, 1, 2),
            (PieceKind::Accessory, 0, 2),
        ],
        SlotKind::Helmet => &[
            (PieceKind::Frame, 1, 1),
            (PieceKind::Plating, 1, 2),
            (PieceKind::Crest, 0, 1),
        ],
        SlotKind::Chest => &[(PieceKind::Base, 1, 1), (PieceKind::Layer, 1, 3)],
        SlotKind::Gloves | SlotKind::Greaves => {
            &[(PieceKind::Material, 1, 1), (PieceKind::Mold, 1, 1)]
        }
    };

    for &(k, min, max) in reqs {
        let have = n(k);
        if have < min {
            return Err(format!("needs {} more {}", min - have, k.name()));
        }
        if have > max {
            return Err(format!("too many {} (max {})", k.name(), max));
        }
    }
    Ok(())
}
