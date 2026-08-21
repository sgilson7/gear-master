use std::collections::HashSet;

use crate::curse::TICK_MS;
use crate::naming::{item_hash, name_item, ItemName};
use crate::piece::{
    default_cooldown_ms, EffectKind, PieceId, PieceKind, PieceRegistry, SlotKind, Solitude,
    Trigger,
};
use crate::slot::{PlaceError, Slot};
use crate::stats::{StatKind, Stats};

/// One spell's payload: what happens on the cast that fires it.
#[derive(Clone, Debug, Default)]
pub struct Cast {
    pub name: String,
    pub stats: Stats,
    pub triggers: Vec<Trigger>,
}

/// One assembled item, reduced to what combat needs: how often it fires and
/// what happens when it does.
#[derive(Clone, Debug)]
pub struct ItemProfile {
    /// Fingerprint of this exact arrangement — the same number the name is
    /// drawn from, so an item's emblem and its name vary together.
    pub sigil_seed: u64,
    /// The components this item is built from, so the interface can find them
    /// on the board — used to shake an item when it fires.
    pub pieces: Vec<PieceId>,
    /// Indices, within this same list, of assembled items touching this one.
    pub adjacent_items: Vec<usize>,
    /// Indices of assembled items in *other* slots lying on the same rows.
    pub aligned_items: Vec<usize>,
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
    /// Hundredths of weapon power that apply to THIS item alone - what the
    /// ink in a spell is worth. Never reaches the wearer's own total.
    pub power_bonus: i32,
    /// For a spell, the payloads it cycles through. A book has one and casts
    /// it every time; a crystal ball has two or three and casts a different
    /// one each time it comes round. Empty for ordinary gear, which carries
    /// its payload on the item itself.
    pub casts: Vec<Cast>,
    /// How effective this arrangement is, on the shared scale in `rating`.
    /// Scored at the cadence the item actually runs at, so speed counts.
    pub rating: i32,
}

impl ItemProfile {
    /// The badge this item has earned.
    pub fn rarity(&self) -> crate::rating::Rarity {
        crate::rating::Rarity::of(self.rating)
    }

    /// What one swing of this item lands for, given the wearer's totals.
    /// Only weapons deal damage; everything else activates for armour, mana
    /// or curses.
    pub fn hit_for(&self, strength: i32, power: i32) -> i32 {
        if self.slot != SlotKind::Weapon {
            return 0;
        }
        (((self.stats.damage + strength) as i64 * power as i64) / 100).max(0) as i32
    }

    /// Damage a second, in thousandths, so a slow heavy weapon and a fast
    /// light one can be compared without floating point.
    pub fn dps_milli(&self, strength: i32, power: i32) -> i64 {
        if self.cooldown_ms == 0 {
            return 0;
        }
        self.hit_for(strength, power) as i64 * 1000 * 1000 / self.cooldown_ms as i64
    }
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
    /// Effectiveness on the shared scale in `rating`. Scored at the slot's
    /// default cadence: a report is about the arrangement, not the fight.
    pub rating: i32,
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

/// An item the player has fixed in place: the exact set of pieces it is made
/// of, and the shape they make.
///
/// The shape has to be carried here rather than read off the board, because a
/// locked item travels as one thing. Once it is lifted into the inventory the
/// board no longer knows how its pieces sat, and without that there is nothing
/// to put back down.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LockedItem {
    pub pieces: Vec<PieceId>,
    /// Anchor of `pieces[i]` relative to the item's own top-left corner.
    /// Kept in step with the board: set when the item locks, and refreshed
    /// whenever it turns.
    pub offsets: Vec<(u8, u8)>,
}

/// The character's five equipment grids.
#[derive(Clone, Debug)]
pub struct Loadout {
    pub slots: Vec<Slot>,
    /// Items the player has fixed in place. Nothing else may join one and it
    /// may not lose a piece to a neighbour.
    pub locks: Vec<LockedItem>,
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
            locks: Vec::new(),
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
        let groups =
            repair_split(slot, reg, kind, slot.items_with_locks(reg, &self.locks), &self.locks);

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
                    if let EffectKind::Flat { stats } = eff.kind {
                        if eff.when.holds(assembled[gi]) {
                            contribution += stats;
                            item_notes.push(format!("{}: {}", def.name, eff.label));
                        }
                    }
                    if let EffectKind::SelfPerNeighborKind { kind: want, stat, per } = eff.kind {
                        if eff.when.holds(assembled[gi]) {
                            let n = slot
                                .neighbors_of(p)
                                .into_iter()
                                .filter(|&q| reg.def(q).kind == want)
                                .count() as i32;
                            if n > 0 {
                                contribution.add(stat, per * n);
                                item_notes.push(format!(
                                    "{}: +{} {} from {} adjacent {}",
                                    def.name,
                                    per * n,
                                    stat.name(),
                                    n,
                                    want.name()
                                ));
                            }
                        }
                    }
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
                rating: if assembled[gi] {
                    crate::rating::item_rating(reg, group, 0, slot.kind)
                } else {
                    0
                },
            });
        }

        SlotReport { slot: kind, items, stats: slot_total }
    }

    /// Activation profiles for every assembled item across every slot — what
    /// combat actually runs on.
    ///
    /// Unassembled groups are deliberately absent: loose pieces still hand over
    /// their passive stats through `report`, but they never act. That is the
    /// cost of leaving gear in bits.
    pub fn combat_items(&self, reg: &PieceRegistry) -> Vec<ItemProfile> {
        // First pass: collect every finished item with the slot it came from.
        let mut gathered: Vec<(SlotKind, GearItem)> = Vec::new();
        for kind in SlotKind::ALL {
            for item in self.report(reg, kind).items {
                if item.assembled {
                    gathered.push((kind, item));
                }
            }
        }

        // Solitude multipliers need every grid at once - "no other item shares
        // a row" is not a question a single slot can answer - so they are
        // resolved here rather than in `report`, which is per-slot.
        let cells: Vec<Vec<(u8, u8)>> = gathered
            .iter()
            .map(|(kind, item)| {
                let slot = self.slot(*kind);
                item.pieces.iter().flat_map(|&p| slot.cells_of(p)).collect()
            })
            .collect();
        let multipliers: Vec<i32> = (0..gathered.len())
            .map(|i| {
                let (kind, item) = &gathered[i];
                let mut times = 1;
                for &p in &item.pieces {
                    let Some(eff) = reg.def(p).effect else { continue };
                    let EffectKind::SoleIf { what, times: n } = eff.kind else { continue };
                    let alone = (0..gathered.len()).filter(|&j| j != i).all(|j| {
                        match what {
                            Solitude::Row => {
                                let rows = |v: &Vec<(u8, u8)>| {
                                    let lo = v.iter().map(|(_, y)| *y).min().unwrap_or(0);
                                    let hi = v.iter().map(|(_, y)| *y).max().unwrap_or(0);
                                    (lo, hi)
                                };
                                let (a0, a1) = rows(&cells[i]);
                                let (b0, b1) = rows(&cells[j]);
                                !(a0 <= b1 && b0 <= a1)
                            }
                            Solitude::Stacked => {
                                !cells[j].iter().any(|c| cells[i].contains(c))
                            }
                            Solitude::StackedWith(want) => {
                                gathered[j].0 != want
                                    || !cells[j].iter().any(|c| cells[i].contains(c))
                            }
                        }
                    });
                    // The piece has to be part of a finished item for its own
                    // effect to count, which it is: `gathered` is assembled
                    // items only.
                    let _ = kind;
                    if alone {
                        times = times.max(n);
                    }
                }
                times
            })
            .collect();

        // Second pass: who touches whom, and who lines up with whom. Both are
        // global indices into the list being built, so combat can resolve a
        // reaction without knowing anything about grids.
        let spans: Vec<Option<(u8, u8)>> = gathered
            .iter()
            .map(|(kind, item)| self.slot(*kind).row_span(&item.pieces))
            .collect();

        let mut out = Vec::with_capacity(gathered.len());
        for (i, (kind, item)) in gathered.iter().enumerate() {
            let slot = self.slot(*kind);
            let mut adjacent = Vec::new();
            let mut aligned = Vec::new();
            for (j, (other_kind, other)) in gathered.iter().enumerate() {
                if i == j {
                    continue;
                }
                if other_kind == kind {
                    if slot.sets_touch(&item.pieces, &other.pieces) {
                        adjacent.push(j);
                    }
                } else if let (Some(a), Some(b)) = (spans[i], spans[j]) {
                    // Different grids: "aligned" means their rows overlap.
                    if a.0 <= b.1 && b.0 <= a.1 {
                        aligned.push(j);
                    }
                }
            }

            let core = item.pieces.iter().copied().find(|&p| reg.def(p).kind.is_core());
            let base_cd = core
                .map(|c| {
                    let d = reg.def(c).cooldown_ms;
                    if d == 0 { default_cooldown_ms(*kind) } else { d }
                })
                .unwrap_or_else(|| default_cooldown_ms(*kind));
            let speed: i32 =
                100 + item.pieces.iter().map(|&p| reg.def(p).speed_bonus).sum::<i32>();
            let speed = speed.max(10);
            let cooldown_ms = ((base_cd as i64 * 100 / speed as i64) as u32).max(TICK_MS);

            let triggers: Vec<Trigger> = item
                .pieces
                .iter()
                .flat_map(|&p| reg.def(p).triggers.iter().copied())
                .collect();

            // Ink scales the cast it is bound into rather than the wearer.
            let power_bonus: i32 = item.pieces.iter().map(|&p| reg.def(p).power_bonus).sum();

            // Every spell in the item is one payload. A book has bound one,
            // an orb several; ordinary gear has none and keeps carrying its
            // payload on the item.
            // An alignment is not cast itself. It colours every spell the ball
            // holds - which is why an orb needs no ink: the alignment is the
            // build decision, and it is a choice of pool rather than a flat
            // multiplier.
            let aligned_by: Vec<PieceId> = item
                .pieces
                .iter()
                .copied()
                .filter(|&p| reg.def(p).kind == PieceKind::Alignment)
                .collect();

            let casts: Vec<Cast> = item
                .pieces
                .iter()
                .filter(|&&p| reg.def(p).kind == PieceKind::Spell)
                .map(|&p| {
                    let d = reg.def(p);
                    let mut stats = d.base;
                    let mut triggers = d.triggers.to_vec();
                    for &a in &aligned_by {
                        let ad = reg.def(a);
                        stats += ad.base;
                        triggers.extend(ad.triggers.iter().copied());
                    }
                    Cast { name: d.name.to_string(), stats, triggers }
                })
                .collect();

            // Everything on the item, multiplied. All the numbers means all of
            // them - what it grants standing, what it does per activation, and
            // every spell it casts.
            let times = multipliers[i];
            let scaled_stats = item.stats.times(times);
            let casts: Vec<Cast> = casts
                .into_iter()
                .map(|c| Cast { stats: c.stats.times(times), ..c })
                .collect();

            out.push(ItemProfile {
                sigil_seed: item_hash(self.name_seed, reg, slot, &item.pieces),
                pieces: item.pieces.clone(),
                adjacent_assembled_same_slot: adjacent.len(),
                adjacent_items: adjacent,
                aligned_items: aligned,
                name: item.name.short.clone(),
                full_name: item.name.full.clone(),
                core: core.map(|c| reg.def(c).name.to_string()).unwrap_or_default(),
                slot: *kind,
                cooldown_ms,
                stats: scaled_stats,
                triggers,
                power_bonus,
                casts,
                rating: crate::rating::item_rating(reg, &item.pieces, cooldown_ms, *kind),
            });
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

/// Hand contested pieces to whichever item actually needs them.
///
/// Items are split by giving each piece to its nearest core. That is the right
/// default, but it is only a proximity rule - it knows nothing about recipes.
/// Pack a spell hard against a weapon and the weapon, being one step closer,
/// can take the spell's ink; both then fail, and the board looks broken for
/// no reason a player can see.
///
/// So after the split, any piece sitting on the boundary between two items is
/// offered to the one that is short of it, provided the item losing it can
/// spare it. Being able to pack tightly is the whole point of having a second
/// recipe in the slot, and this is what makes it safe.
fn repair_split(
    slot: &Slot,
    reg: &PieceRegistry,
    kind: SlotKind,
    mut groups: Vec<Vec<PieceId>>,
    locks: &[LockedItem],
) -> Vec<Vec<PieceId>> {
    let is_locked = |g: &Vec<PieceId>| locks.iter().any(|l| l.pieces == *g);
    // A handful of passes is plenty: each one either fixes an item or changes
    // nothing, and a slot never holds many items.
    for _ in 0..4 {
        let ok: Vec<bool> =
            groups.iter().map(|g| check_recipe(kind, reg, g).is_ok()).collect();
        if ok.iter().all(|v| *v) {
            break;
        }
        let mut moved = false;
        'outer: for want in 0..groups.len() {
            // A locked item neither takes nor gives.
            if ok[want] || is_locked(&groups[want]) {
                continue;
            }
            for give in 0..groups.len() {
                if give == want || groups[give].len() <= 1 || is_locked(&groups[give]) {
                    continue;
                }
                for (pos, &piece) in groups[give].iter().enumerate() {
                    // Only pieces actually touching the needy item, or it
                    // would end up with parts it is not connected to.
                    if !slot.sets_touch(&[piece], &groups[want]) {
                        continue;
                    }
                    let mut donor = groups[give].clone();
                    donor.remove(pos);
                    let mut taker = groups[want].clone();
                    taker.push(piece);
                    // Only if it helps the one and does not break the other.
                    if check_recipe(kind, reg, &taker).is_ok()
                        && check_recipe(kind, reg, &donor).is_ok()
                        && slot.connected(&donor)
                    {
                        groups[give] = donor;
                        groups[want] = taker;
                        moved = true;
                        break 'outer;
                    }
                }
            }
        }
        if !moved {
            break;
        }
    }
    groups
}

/// Does this group of components satisfy the slot's recipe? Returns the
/// missing-requirement message on failure, phrased for the player. Counts are
/// per item, not per slot — two complete weapons in one slot is legal.
fn check_recipe(kind: SlotKind, reg: &PieceRegistry, pieces: &[PieceId]) -> Result<(), String> {
    let counts = Slot::kind_counts(reg, pieces);
    let n = |k: PieceKind| counts.get(&k).copied().unwrap_or(0);

    // A slot can offer several recipes - the weapon slot builds either a
    // martial weapon or a spell - and satisfying any one of them is enough.
    let mut best: Option<(usize, String)> = None;
    for recipe in crate::piece::recipes(kind) {
        let mut problem = None;
        // How much of this recipe the pieces already answer to, so the message
        // on failure comes from whichever one they were closest to building.
        let mut matched = 0usize;
        for &(k, min, max) in *recipe {
            let have = n(k);
            matched += have.min(max);
            if problem.is_none() {
                if have < min {
                    problem = Some(format!("needs {} more {}", min - have, k.name()));
                } else if have > max {
                    problem = Some(format!("too many {} (max {})", k.name(), max));
                }
            }
        }
        // Anything not named by this recipe does not belong in it.
        let named: usize = recipe
            .iter()
            .map(|&(k, _, max)| n(k).min(max))
            .sum();
        if problem.is_none() && named < pieces.len() {
            problem = Some(String::from("has parts that do not belong together"));
        }
        match problem {
            None => return Ok(()),
            Some(msg) => {
                if best.as_ref().map(|(m, _)| matched > *m).unwrap_or(true) {
                    best = Some((matched, msg));
                }
            }
        }
    }
    Err(best.map(|(_, m)| m).unwrap_or_else(|| String::from("nothing fits a recipe")))
}
