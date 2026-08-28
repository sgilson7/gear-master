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
    /// Indices, within this same list, of assembled items meeting this one at
    /// a corner and along no edge. Same grid, and never also adjacent.
    pub diagonal_items: Vec<usize>,
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
    /// Empty cells touching this item - what `PerAdjacentEmpty` repeats over.
    pub open_cells: usize,
    /// **Overtake**: the first time this item fires in a fight, it fires
    /// again immediately. Read off the pieces here so combat does not have to
    /// walk a registry it does not have.
    pub overtakes: bool,
    /// **The wrong sense.** This item's board deals no physical and no magic,
    /// and its mind damage is multiplied by what it gave up. One crest carries
    /// it, and it is the board's rather than the item's - which is why combat
    /// reads it off every profile and sets it once.
    pub wrong_sense: bool,
    /// Whether a misfire eats this item's activation.
    ///
    /// One piece in the game says no - a Stray Orb, whose spells go off
    /// whatever the curse says. Per item rather than per fighter, because that
    /// is what makes it a decision about which item to build the orb into
    /// rather than a flat immunity somebody bought.
    pub steady: bool,
    /// Whether this item is standing on a Lightning Rod.
    ///
    /// Curses that pick a target on your board pick this one instead. Which
    /// makes the rod a decision rather than a reward: you lay it under
    /// something you do not mind losing the use of, and everything you do mind
    /// stops being picked.
    pub attracts_curses: bool,
    /// Hundredths of weapon power that apply to THIS item alone - what the
    /// ink in a spell is worth. Never reaches the wearer's own total.
    pub power_bonus: i32,
    /// What this item multiplies its own damage by, in hundredths. 100 is
    /// plain.
    ///
    /// Every point of power on every piece of the item, plus its ink, and
    /// nothing from anywhere else. Power used to be a wearer stat: a helmet
    /// with power on it multiplied the *weapon*, so a build could stack power
    /// in five slots at once and the same blade would hit for many times what
    /// it does here. Strength is the stat that reaches across the build now,
    /// and it is the only one.
    pub power: i32,
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
    /// Takes strength because strength is the one stat that reaches across a
    /// build. The multiplier is the item's own.
    pub fn hit_for(&self, strength: i32) -> i32 {
        if self.slot != SlotKind::Weapon {
            return 0;
        }
        (((self.stats.physical_damage + self.stats.magic_damage + strength) as i64
            * self.power as i64)
            / 100)
            .max(0) as i32
    }

    /// Damage a second, in thousandths, so a slow heavy weapon and a fast
    /// light one can be compared without floating point.
    pub fn dps_milli(&self, strength: i32) -> i64 {
        if self.cooldown_ms == 0 {
            return 0;
        }
        self.hit_for(strength) as i64 * 1000 * 1000 / self.cooldown_ms as i64
    }
}

/// Fold **Commons** into one item's neighbour lists.
///
/// A commons item is adjacent to every assembled item on the board and every
/// assembled item is adjacent to it, so either end of the relation puts the
/// pair in each other's lists - `commons[i] || commons[j]`, not `&&`, and not
/// `commons[i]` alone, which would be a one-way adjacency and a different
/// mechanic wearing this one's name.
///
/// Split out because F5 lands the effect and F6 lands the pieces that carry
/// it, so this is the only part of the rule a test can reach until then - and
/// because the two things that go wrong here are counting a real neighbour
/// twice and leaving an item in `diagonal_items` that is now adjacent, both of
/// which are invisible in a board test and obvious in this one.
pub fn join_the_commons(
    i: usize,
    commons: &[bool],
    adjacent: &mut Vec<usize>,
    diagonal: &mut Vec<usize>,
) {
    if commons.is_empty() {
        return;
    }
    for j in 0..commons.len() {
        if j != i && (commons[i] || commons[j]) {
            adjacent.push(j);
        }
    }
    adjacent.sort_unstable();
    // A neighbour cannot be a neighbour twice, and a commons item that also
    // genuinely touches something must not be counted once for each reason.
    adjacent.dedup();
    // `diagonal_items` is documented as "never also adjacent". Commons makes
    // corners into edges, and this is what keeps the promise.
    diagonal.retain(|j| !adjacent.contains(j));
}

/// Whether **Bearing** pays: the item carries it, and its slot holds no other
/// assembled item.
///
/// Counted, not overlapped. Two greaves items that never touch are both alone
/// under `Solitude::StackedWith` and neither is alone under this.
pub fn bearing_doubles(carries_bearing: bool, others_assembled_in_slot: usize) -> bool {
    carries_bearing && others_assembled_in_slot == 0
}

/// What THE HUNDRED's six tolls read off a board.
///
/// **Derived figures, never raw stats.** A toll asks what a board *does* a
/// second, which is the question a river and a ford and a scarp are all
/// versions of, and it is a different question from what a board *has*. The
/// worked pair A3 exists for: eight mana on a four-second item pays 2,000
/// milli-mana a second and three mana on a one-second item pays 3,000, so the
/// worse-looking piece crosses the deeper river.
///
/// Everything is in **milli-units a second** - a thousandth of a point a
/// second - with the division done per item and then summed, which is not the
/// same as summing and then dividing and is the shape that makes a fast item
/// worth what it is worth. No float touches any of it.
///
/// Over **assembled items only**. A loose piece contributes passive stats and
/// does not act, and a toll is about acting.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Figures {
    /// Mana a second, in thousandths. The river.
    pub flow: i64,
    /// Flat physical damage a second, in thousandths. One of the two fords.
    pub physical_dps: i64,
    /// Flat magic damage a second, in thousandths. The other.
    pub magic_dps: i64,
    /// Armour a second, in thousandths. The scarp.
    pub armour_ps: i64,
    /// The fastest assembled item, in milliseconds. The drift.
    ///
    /// `None` when nothing is assembled, which is not the same as slow: a
    /// board with no items has no fastest one, and a drift asks for a board
    /// that acts *often* rather than for a board.
    pub fastest_ms: Option<u32>,
    /// Summed curse resistance across assembled items. The hedge.
    ///
    /// The one figure that is a stat rather than a rate, because curse
    /// resistance is a percentage held rather than a thing paid out - and a
    /// hedge is a fence you are proof against rather than one you outrun.
    pub curse_resist: i32,
}

/// One item's contribution to a per-second figure, in thousandths.
///
/// `stat * 1_000_000 / cooldown_ms`: a point per activation on a 1,000 ms item
/// is one a second, which is 1,000 milli-units. The same arithmetic
/// `ItemProfile::dps_milli` has always used for weapon damage.
fn per_second_milli(stat: i32, cooldown_ms: u32) -> i64 {
    if cooldown_ms == 0 {
        return 0;
    }
    stat as i64 * 1_000_000 / cooldown_ms as i64
}

impl Figures {
    /// Read a board's six figures off its assembled items.
    pub fn of(items: &[ItemProfile]) -> Figures {
        let mut f = Figures::default();
        for i in items {
            f.flow += per_second_milli(i.stats.mana, i.cooldown_ms);
            f.physical_dps += per_second_milli(i.stats.physical_damage, i.cooldown_ms);
            f.magic_dps += per_second_milli(i.stats.magic_damage, i.cooldown_ms);
            f.armour_ps += per_second_milli(i.stats.armor, i.cooldown_ms);
            f.curse_resist += i.stats.curse_resist;
            if i.cooldown_ms > 0 {
                f.fastest_ms = Some(f.fastest_ms.map_or(i.cooldown_ms, |m| m.min(i.cooldown_ms)));
            }
        }
        f
    }

    /// The damage figure a ford in one lane asks for.
    pub fn dps(&self, lane: crate::county::Lane) -> i64 {
        match lane {
            crate::county::Lane::Physical => self.physical_dps,
            crate::county::Lane::Magic => self.magic_dps,
        }
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

/// How fast an item actually runs: its core's cadence, quickened by every
/// speed bonus its pieces carry.
///
/// One function because there used to be two answers. `combat_items` worked it
/// out properly and `report` passed zero, so an item's rating - and therefore
/// its rarity, and therefore how many words its name got - was computed
/// against the slot's default cadence rather than its own. Twelve percent of
/// the items in the game came out a whole tier apart depending on which one
/// you asked, which is how a legendary ended up with a three-word name.
pub fn item_cooldown_ms(reg: &PieceRegistry, pieces: &[PieceId], kind: SlotKind) -> u32 {
    let core = pieces.iter().copied().find(|&p| reg.def(p).kind.is_core());
    let base = core
        .map(|c| {
            let d = reg.def(c).cooldown_ms;
            if d == 0 { default_cooldown_ms(kind) } else { d }
        })
        .unwrap_or_else(|| default_cooldown_ms(kind));
    let speed = (100 + pieces.iter().map(|&p| reg.def(p).speed_bonus).sum::<i32>()).max(10);
    ((base as i64 * 100 / speed as i64) as u32).max(TICK_MS)
}

/// Fix every assembled item in one slot where it stands.
///
/// What locking buys, and why a monster's board uses it: an unlocked item
/// negotiates with whatever it is touching. Pack two of them flush and the
/// optional pieces drift to whichever core is nearest, so the arrangement you
/// authored is not necessarily the arrangement that comes out. Locked, each is
/// a single large component - which means a board can be packed far tighter
/// than one that has to leave gaps to stay legible.
///
/// Returns how many items it fixed.
pub fn lock_assembled_in(
    loadout: &mut Loadout,
    reg: &PieceRegistry,
    slot: SlotKind,
) -> usize {
    let items: Vec<Vec<PieceId>> = loadout
        .report(reg, slot)
        .items
        .into_iter()
        .filter(|i| i.assembled)
        .map(|i| i.pieces)
        .collect();
    let mut n = 0;
    for pieces in items {
        if pieces.iter().any(|p| loadout.locks.iter().any(|l| l.pieces.contains(p))) {
            continue;
        }
        let g = loadout.slot(slot);
        let anchors: Vec<(u8, u8)> =
            pieces.iter().map(|&p| g.anchor_of(p).unwrap_or((0, 0))).collect();
        let minx = anchors.iter().map(|(x, _)| *x).min().unwrap_or(0);
        let miny = anchors.iter().map(|(_, y)| *y).min().unwrap_or(0);
        let offsets = anchors.iter().map(|&(x, y)| (x - minx, y - miny)).collect();
        loadout.locks.push(LockedItem { pieces, offsets });
        n += 1;
    }
    n
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
    /// The words items are named out of. A display concern like the rest of
    /// the theme, but it has to live here: names are generated where items
    /// are, not where they are drawn.
    pub naming: &'static crate::naming::Naming,
    /// Extra percent on every assembly bonus, from Recycler.
    ///
    /// It lives on the loadout rather than being passed in because `report` is
    /// called from a hundred and eight places and every one of them - the
    /// character sheet, each item card, the shop's comparison, combat - has to
    /// see the same number. A parameter through all of them would be a
    /// parameter somebody forgets in one place, and the bug that makes is an
    /// item card that disagrees with the fight.
    ///
    /// `Run::refresh_class_effects` is the only thing that writes it.
    pub assembly_pct: i32,
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
            naming: &crate::naming::PLAIN_NAMING,
            assembly_pct: 0,
        }
    }

    /// Add rows to every grid, keeping everything where it is.
    ///
    /// All five together, because the thing that grants this grants it to the
    /// whole board - a run where one slot is taller than the others would be a
    /// different game, and a much more confusing one.
    pub fn grow(&mut self, by: u8) {
        for s in self.slots.iter_mut() {
            s.grow(by);
        }
    }

    /// Grow one grid and leave the other four where they are.
    ///
    /// `branching-events.md` says a run where one slot is taller than the
    /// others "would be a different game and a much more confusing one", and
    /// that was the right rule while the only thing handing out room was
    /// Sprocketman's Gratitude, which hands out five. The Depth hands out one,
    /// on a board of your choice, and the choice is the reward - so the rule
    /// is amended rather than worked around. `Slot` has carried its own height
    /// since the day rows became a thing; this is the first caller to use it.
    pub fn grow_one(&mut self, kind: SlotKind, by: u8) {
        self.slots[kind.index()].grow(by);
    }

    /// How tall the tallest grid is.
    ///
    /// It used to be "every slot is the same height", and for layout that is
    /// still the number worth having - a row of boards is as tall as its
    /// tallest. Anything asking whether a *placement* fits must ask the slot,
    /// not this.
    pub fn rows(&self) -> u8 {
        self.slots.iter().map(|s| s.rows()).max().unwrap_or(crate::slot::SLOT_H)
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
                    if let Some(adj) = reg.def(p).assembly_bonus {
                        // Recycler counts the bonus for more than it says on
                        // the piece. The label still quotes the printed
                        // number, so the note says what the component is and
                        // the total says what it came to.
                        item_stats += adj.stats.scaled(100 + self.assembly_pct);
                        item_notes.push(adj.label.to_string());
                    }
                }
            }
            slot_total += item_stats;
            // A name grows with what the item is worth, so the rating has to
            // be in hand before the name is made.
            // At the cadence it will actually run at, not the slot's default.
            // Passing zero here is what made a quickened item's name disagree
            // with its own badge.
            let rating = if assembled[gi] {
                let cd = item_cooldown_ms(reg, group, slot.kind);
                crate::rating::item_rating(reg, group, cd, slot.kind)
            } else {
                0
            };
            items.push(GearItem {
                name: name_item(
                    self.name_seed,
                    reg,
                    slot,
                    group,
                    crate::rating::Rarity::of(rating),
                    self.naming,
                ),
                pieces: group.clone(),
                assembled: assembled[gi],
                status: match &verdicts[gi] {
                    Ok(()) => "assembled".to_string(),
                    Err(reason) => reason.clone(),
                },
                stats: item_stats,
                notes: item_notes,
                rating,
            });
        }

        // 6. Enchantments. One is never in a group - `groups` walks the gear
        //    layer and an enchantment is under it - so it contributes here, on
        //    its own, as the permanently-loose thing it is. Its stats reach the
        //    wearer the same way any unassembled piece's do.
        for id in slot.pieces() {
            let def = reg.def(id);
            if !def.kind.is_enchantment() {
                continue;
            }
            // Dead enchantments give nothing at all, stats included. An
            // enchantment with another one touching it is not a weaker
            // enchantment, it is a smothered one.
            let live = slot.enchant_is_live(id);
            let mut contribution = if live { def.base } else { Stats::ZERO };
            let mut item_notes: Vec<String> = Vec::new();
            if !live {
                item_notes.push(format!("{}: smothered - another enchantment touches it", def.name));
            }
            if let Some(eff) = def.effect {
                let covering = slot.covering(id);
                let n = match eff.kind {
                    EffectKind::PerOverlappingItem { .. } => covering.len() as i32,
                    EffectKind::PerOverlappingCore { .. } => {
                        covering.iter().filter(|&&c| reg.def(c).kind.is_core()).count() as i32
                    }
                    _ => 0,
                };
                if let EffectKind::PerOverlappingItem { stat, amount }
                | EffectKind::PerOverlappingCore { stat, amount } = eff.kind
                {
                    // An enchantment is never assembled, so `When::Assembled`
                    // on one would silence it for ever. `holds(false)` is the
                    // honest question and says so.
                    if live && n > 0 && eff.when.holds(false) {
                        contribution.add(stat, amount * n);
                        item_notes.push(format!(
                            "{}: +{} {} from {} covering it",
                            def.name,
                            amount * n,
                            stat.name(),
                            n
                        ));
                    }
                }
            }
            slot_total += contribution;
            items.push(GearItem {
                name: name_item(
                    self.name_seed,
                    reg,
                    slot,
                    &[id],
                    crate::rating::Rarity::of(0),
                    self.naming,
                ),
                pieces: vec![id],
                assembled: false,
                status: if !live {
                    "smothered".to_string()
                } else if slot.enchant_is_buried(id) {
                    "bonded".to_string()
                } else {
                    "enchantment".to_string()
                },
                stats: contribution,
                notes: item_notes,
                rating: 0,
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
                // Bearing: double while this is the only assembled item in its
                // slot. Folded in here because "this item's stats count n
                // times" is what this pass computes, and a second pass that
                // multiplied stats somewhere else would be a second answer to
                // one question.
                //
                // Counted, not overlapped. Two greaves items that never touch
                // are both alone under `Solitude::StackedWith` and neither is
                // alone under this.
                let bears = item
                    .pieces
                    .iter()
                    .any(|&p| matches!(reg.def(p).effect.map(|e| e.kind), Some(EffectKind::Bearing)));
                let others_here =
                    gathered.iter().enumerate().filter(|(j, (k, _))| *j != i && k == kind).count();
                if bearing_doubles(bears, others_here) {
                    times = times.max(2);
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

        // Commons: an item that counts as adjacent to every assembled item on
        // the board, and they to it. Computed before the pass below rather
        // than inside it, because the relation is symmetric and a pass that
        // only ever looks at `i` can only make it one-way - which would be a
        // different mechanic wearing this one's name.
        let commons: Vec<bool> = gathered
            .iter()
            .map(|(_, item)| {
                item.pieces
                    .iter()
                    .any(|&p| matches!(reg.def(p).effect.map(|e| e.kind), Some(EffectKind::Commons)))
            })
            .collect();

        let mut out = Vec::with_capacity(gathered.len());
        for (i, (kind, item)) in gathered.iter().enumerate() {
            let slot = self.slot(*kind);
            let mut adjacent = Vec::new();
            let mut aligned = Vec::new();
            let mut diagonal = Vec::new();
            for (j, (other_kind, other)) in gathered.iter().enumerate() {
                if i == j {
                    continue;
                }
                if other_kind == kind {
                    if slot.sets_touch(&item.pieces, &other.pieces) {
                        adjacent.push(j);
                    } else if slot.sets_touch_diagonally(&item.pieces, &other.pieces) {
                        diagonal.push(j);
                    }
                } else if let (Some(a), Some(b)) = (spans[i], spans[j]) {
                    // Different grids: "aligned" means their rows overlap.
                    if a.0 <= b.1 && b.0 <= a.1 {
                        aligned.push(j);
                    }
                }
            }
            join_the_commons(i, &commons, &mut adjacent, &mut diagonal);

            let core = item.pieces.iter().copied().find(|&p| reg.def(p).kind.is_core());
            let cooldown_ms = item_cooldown_ms(reg, &item.pieces, *kind);

            // A piece's own triggers, and the ones its assembly bonus lends
            // the item.
            //
            // No `assembled` test here, and none is needed: `combat_items`
            // filtered to finished items before this loop began, so anything
            // reaching this line is already assembled. That is the whole of
            // "only while assembled" - the condition is the function.
            let raw_triggers: Vec<Trigger> = item
                .pieces
                .iter()
                .flat_map(|&p| {
                    let d = reg.def(p);
                    d.triggers.iter().copied().chain(
                        d.assembly_bonus.into_iter().flat_map(|b| b.triggers.iter().copied()),
                    )
                })
                .collect();

            // Ink scales the cast it is bound into rather than the wearer.
            let power_bonus: i32 = item.pieces.iter().map(|&p| reg.def(p).power_bonus).sum();
            // And so does power, now. Base is a plain multiple of one.
            let mut power: i32 =
                100 + item.pieces.iter().map(|&p| reg.def(p).base.power).sum::<i32>() + power_bonus;

            // The bond. An enchantment this item is built on top of doubles it
            // and hands it a trigger.
            //
            // Two conditions, one on each layer, and they pull opposite ways.
            // The enchantment has to be *live* - nothing else on its own layer
            // touching it - which wants enchantments spread out. And it has to
            // be *buried* by this item alone: every one of its cells covered,
            // and every covering piece part of this item, which wants gear
            // packed tight and shaped to fit. An item that happens to cover
            // half of one gets nothing, and two items sharing the cover get
            // nothing either.
            //
            // The payout is `+1.00x power`, and power is already the thing that
            // multiplies an item's stats and what its triggers pay out (never
            // what they cost) - so doubling it means the same thing in all five
            // grids rather than only in the one that swings.
            let mut raw_triggers = raw_triggers;
            let mut attracts_curses = false;
            for eid in slot.pieces() {
                if !reg.def(eid).kind.is_enchantment() {
                    continue;
                }
                if !slot.enchant_is_live(eid) {
                    continue;
                }
                let cells = slot.enchant_cells(eid);
                let bonded = !cells.is_empty()
                    && cells.iter().all(|&(x, y)| match slot.get(x, y) {
                        Some(on_top) => item.pieces.contains(&on_top),
                        None => false,
                    });
                if bonded {
                    power += 100;
                    raw_triggers.extend(reg.def(eid).triggers.iter().copied());
                }
                // The rod asks for less than the bond does: *covering* it is
                // enough, and covering all of it is not required. A rod
                // half-under something still has a wire running up into it.
                if reg.def(eid).name == crate::piece::LIGHTNING_ROD {
                    attracts_curses |= cells.iter().any(|&(x, y)| {
                        slot.get(x, y).is_some_and(|on_top| item.pieces.contains(&on_top))
                    });
                }
            }
            let power = power;

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
            // And then the item's own power on top, which multiplies its
            // numbers and what its triggers pay out - but never what they
            // cost. Baked in here rather than at the point of use, so combat,
            // the card and the rating are all reading the same figures.
            let scaled_stats = item.stats.times(times).powered(power);
            let casts: Vec<Cast> = casts
                .into_iter()
                .map(|c| Cast {
                    stats: c.stats.times(times).powered(power),
                    triggers: c.triggers.into_iter().map(|t| t.scaled(power)).collect(),
                    ..c
                })
                .collect();
            let triggers: Vec<Trigger> =
                raw_triggers.into_iter().map(|t| t.scaled(power)).collect();

            out.push(ItemProfile {
                sigil_seed: item_hash(self.name_seed, reg, slot, &item.pieces),
                pieces: item.pieces.clone(),
                adjacent_assembled_same_slot: adjacent.len(),
                open_cells: slot.open_cells_around(&item.pieces),
                attracts_curses,
                steady: item
                    .pieces
                    .iter()
                    .any(|&p| reg.def(p).name == crate::piece::STRAY_ORB),
                overtakes: item.pieces.iter().any(|&p| {
                    matches!(reg.def(p).effect.map(|e| e.kind), Some(EffectKind::Overtake))
                }),
                // The trade, read off the board the same way. A standing state
                // rather than a trigger, so it is true from the bell.
                wrong_sense: item.pieces.iter().any(|&p| {
                    matches!(reg.def(p).effect.map(|e| e.kind), Some(EffectKind::WrongSense))
                }),
                adjacent_items: adjacent,
                aligned_items: aligned,
                diagonal_items: diagonal,
                name: item.name.short.clone(),
                full_name: item.name.full.clone(),
                core: core.map(|c| reg.def(c).name.to_string()).unwrap_or_default(),
                slot: *kind,
                cooldown_ms,
                stats: scaled_stats,
                triggers,
                power_bonus,
                power,
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
