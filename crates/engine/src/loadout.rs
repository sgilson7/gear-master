use std::collections::HashSet;

use crate::piece::{EffectKind, PieceId, PieceKind, PieceRegistry, SlotKind};
use crate::slot::{PlaceError, Slot};
use crate::stats::{StatKind, Stats};

/// One orthogonally-connected group of components inside a slot — a candidate
/// piece of gear. A slot can hold as many of these as the player can fit
/// without them touching.
#[derive(Clone, Debug)]
pub struct GearItem {
    pub pieces: Vec<PieceId>,
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
}

impl Default for Loadout {
    fn default() -> Self {
        Self::new()
    }
}

impl Loadout {
    pub fn new() -> Self {
        Loadout { slots: SlotKind::ALL.iter().map(|&k| Slot::new(k)).collect() }
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
    ///   1. split the slot into connected groups
    ///   2. decide which groups satisfy the recipe (nothing has contributed
    ///      stats yet, so this can't depend on effect results)
    ///   3. total each group's stats, applying effects against the assembly
    ///      answers from step 2
    ///   4. add the flat assembly bonuses of assembled groups
    pub fn report(&self, reg: &PieceRegistry, kind: SlotKind) -> SlotReport {
        let slot = self.slot(kind);
        let groups = slot.items(reg);

        // 2. Assembly verdicts, one per group.
        let verdicts: Vec<Result<(), String>> =
            groups.iter().map(|g| check_recipe(kind, reg, g)).collect();

        // Which group each piece belongs to, so an effect can be checked
        // against its own group's verdict.
        let mut group_of: Vec<(PieceId, usize)> = Vec::new();
        for (gi, g) in groups.iter().enumerate() {
            for &p in g {
                group_of.push((p, gi));
            }
        }
        let assembled_of = |id: PieceId| -> bool {
            group_of
                .iter()
                .find(|(p, _)| *p == id)
                .map(|&(_, gi)| verdicts[gi].is_ok())
                .unwrap_or(false)
        };

        let mut items = Vec::new();
        let mut slot_total = Stats::ZERO;

        for (gi, group) in groups.iter().enumerate() {
            let assembled = verdicts[gi].is_ok();
            let mut item_stats = Stats::ZERO;
            let mut notes: Vec<String> = Vec::new();

            // 3. Per-piece contribution: base, then self effects, then any
            //    doubling coming from neighbours.
            for &p in group {
                let def = reg.def(p);
                let mut contribution = def.base;

                if let Some(eff) = def.effect {
                    if let EffectKind::SelfPerEmptyCell { stat, per } = eff.kind {
                        if eff.when.holds(assembled) {
                            let n = slot.empty_neighbor_cells(p) as i32;
                            if n > 0 {
                                contribution.add(stat, per * n);
                                notes.push(format!(
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

                // A neighbour is by definition in the same group, but check
                // each source's own condition anyway so the rule stays local.
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
                        notes.push(format!(
                            "{}: {} doubled to {}",
                            def.name,
                            stat.name(),
                            before * 2
                        ));
                    }
                }

                item_stats += contribution;
            }

            // 4. Flat assembly bonuses, only for a finished item.
            if assembled {
                for &p in group {
                    if let Some(adj) = reg.def(p).adjacency {
                        item_stats += adj.stats;
                        notes.push(adj.label.to_string());
                    }
                }
            }

            slot_total += item_stats;
            items.push(GearItem {
                pieces: group.clone(),
                assembled,
                status: match &verdicts[gi] {
                    Ok(()) => "assembled".to_string(),
                    Err(reason) => reason.clone(),
                },
                stats: item_stats,
                notes,
            });
        }

        SlotReport { slot: kind, items, stats: slot_total }
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
