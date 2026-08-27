//! A search that lays a set of components out in a slot so that every item in
//! it assembles. Used to author the bosses: hand-placing polyominoes and
//! hoping the core-anchoring split falls the right way is a good way to ship a
//! boss whose gear silently does nothing.
//!
//! `cargo test -p gearmaster-engine --test packing -- --ignored --nocapture`
//! prints gear tuples ready to paste into `LADDER`.

use gearmaster_engine::loadout::{lock_assembled_in, Loadout};
use gearmaster_engine::piece::{PieceId, PieceKind, PieceRegistry, SlotKind, Trigger, CATALOG};
use gearmaster_engine::slot::{SLOT_H, SLOT_W};

const CELLS: usize = SLOT_W as usize * SLOT_H as usize;

struct Packer<'a> {
    slot: SlotKind,
    names: &'a [&'static str],
    ids: Vec<PieceId>,
    /// Every cell must end up covered, which allows much sharper pruning.
    exact: bool,
    placed: Vec<(&'static str, u8, u8, u8)>,
}

impl<'a> Packer<'a> {
    /// Rotations that actually change the footprint, so a square piece is not
    /// tried four times.
    fn distinct_rotations(reg: &mut PieceRegistry, id: PieceId) -> Vec<u8> {
        let mut seen: Vec<Vec<(i8, i8)>> = Vec::new();
        let mut out = Vec::new();
        for rot in 0..4u8 {
            reg.set_rotation(id, rot);
            let cells = reg.shape(id).cells().to_vec();
            if !seen.contains(&cells) {
                seen.push(cells);
                out.push(rot);
            }
        }
        out
    }

    fn first_empty(loadout: &Loadout, slot: SlotKind) -> Option<(u8, u8)> {
        for y in 0..SLOT_H {
            for x in 0..SLOT_W {
                if loadout.slot(slot).get(x, y).is_none() {
                    return Some((x, y));
                }
            }
        }
        None
    }

    fn touches_placed(loadout: &Loadout, slot: SlotKind, reg: &PieceRegistry, id: PieceId,
                      ax: u8, ay: u8) -> bool {
        for &(dx, dy) in reg.shape(id).cells() {
            let (cx, cy) = (ax as i32 + dx as i32, ay as i32 + dy as i32);
            for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
                if (0..SLOT_W as i32).contains(&nx) && (0..SLOT_H as i32).contains(&ny) {
                    if loadout.slot(slot).get(nx as u8, ny as u8).is_some() {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn search(&mut self, reg: &mut PieceRegistry, loadout: &mut Loadout, used: &mut Vec<bool>) -> bool {
        if used.iter().all(|u| *u) {
            let report = loadout.report(reg, self.slot);
            return !report.items.is_empty() && report.items.iter().all(|it| it.assembled);
        }
        // In an exact fill, the first empty cell has to be covered by
        // something, so try every unplaced piece against that one cell. Fixing
        // the piece order instead - the obvious thing - is not a valid
        // pruning: it wrongly rejects layouts where a later piece is the only
        // one that fits the corner.
        let must_cover = if self.exact { Self::first_empty(loadout, self.slot) } else { None };

        for idx in 0..self.ids.len() {
            if used[idx] {
                continue;
            }
            // Outside an exact fill the order is fixed, so the search does not
            // waste time on permutations of the same layout.
            if !self.exact && used[..idx].iter().any(|u| !*u) {
                continue;
            }
            let id = self.ids[idx];
            let placed_before = self.placed.len();
            for rot in Self::distinct_rotations(reg, id) {
                reg.set_rotation(id, rot);
                let candidates: Vec<(u8, u8)> = match must_cover {
                    Some((tx, ty)) => reg
                        .shape(id)
                        .cells()
                        .iter()
                        .filter_map(|&(dx, dy)| {
                            let (ax, ay) = (tx as i32 - dx as i32, ty as i32 - dy as i32);
                            (ax >= 0 && ay >= 0).then_some((ax as u8, ay as u8))
                        })
                        .collect(),
                    None => (0..SLOT_H)
                        .flat_map(|y| (0..SLOT_W).map(move |x| (x, y)))
                        .collect(),
                };
                for (x, y) in candidates {
                    if loadout.can_place(reg, id, self.slot, x, y).is_err() {
                        continue;
                    }
                    // Keep everything in one blob: a scattered layout is never
                    // what a boss wants, and it prunes hard.
                    if !self.exact
                        && placed_before > 0
                        && !Self::touches_placed(loadout, self.slot, reg, id, x, y)
                    {
                        continue;
                    }
                    loadout.slot_mut(self.slot).place(reg, id, x, y);
                    self.placed.push((self.names[idx], x, y, rot));
                    used[idx] = true;
                    if self.search(reg, loadout, used) {
                        return true;
                    }
                    used[idx] = false;
                    self.placed.pop();
                    loadout.slot_mut(self.slot).remove(id);
                }
            }
        }
        false
    }
}

fn pack(slot: SlotKind, names: &[&'static str]) -> Option<Vec<(&'static str, u8, u8, u8)>> {
    let mut reg = PieceRegistry::new();
    let mut ids = Vec::new();
    for n in names {
        let d = CATALOG
            .iter()
            .position(|c| c.name == *n)
            .unwrap_or_else(|| panic!("no component named {}", n));
        // `fits`, not `slot ==`: a shared material or plating belongs to one
        // slot but is wearable in another, and this guard is only here to
        // catch a piece named into a slot that cannot hold it at all.
        assert!(CATALOG[d].fits(slot), "{} does not fit in {}", n, slot.name());
        ids.push(reg.alloc(d));
    }
    let used: usize = names
        .iter()
        .map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len())
        .sum();

    // Largest first: big awkward pieces placed early fail fast.
    let mut order: Vec<usize> = (0..ids.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(reg.shape(ids[i]).cells().len()));
    let ordered_ids: Vec<PieceId> = order.iter().map(|&i| ids[i]).collect();
    let ordered_names: Vec<&'static str> = order.iter().map(|&i| names[i]).collect();

    let mut packer = Packer {
        slot,
        names: &ordered_names,
        ids: ordered_ids,
        exact: used == CELLS,
        placed: Vec::new(),
    };
    let mut loadout = Loadout::new();
    let mut used = vec![false; packer.ids.len()];
    if packer.search(&mut reg, &mut loadout, &mut used) {
        Some(packer.placed)
    } else {
        None
    }
}

fn emit(label: &str, slot: SlotKind, names: &[&'static str]) {
    let used: usize = names
        .iter()
        .map(|n| CATALOG.iter().find(|c| c.name == *n).expect("known").cells.len())
        .sum();
    match pack(slot, names) {
        None => println!("// {} {}: NO PACKING FOUND", label, slot.name()),
        Some(p) => {
            println!("// {} {} - {} of {} cells", label, slot.name(), used, CELLS);
            for (n, x, y, r) in p {
                println!("            (\"{}\", SlotKind::{:?}, {}, {}, {}),", n, slot, x, y, r);
            }
        }
    }
}

/// Every legal component multiset for a slot, best-rated first.
///
/// Repeats are allowed: a boss is not shopping, and nothing in the rules says
/// two of the same layer cannot go on one chestpiece.
fn candidates(slot: SlotKind) -> Vec<(i32, Vec<&'static str>)> {
    // Every recipe the slot offers, not just the first: the weapon slot builds
    // martial weapons, book spells and orb spells, and only generating the
    // first meant no monster and no analysis ever saw a spell.
    let mut out: Vec<(i32, Vec<&'static str>)> = Vec::new();
    for recipe in gearmaster_engine::piece::recipes(slot) {
        out.extend(candidates_for(slot, recipe));
    }
    out.sort_by_key(|(r, _)| std::cmp::Reverse(*r));
    out
}

fn candidates_for(
    slot: SlotKind,
    recipe: &'static [(PieceKind, usize, usize)],
) -> Vec<(i32, Vec<&'static str>)> {
    use gearmaster_engine::rating::piece_rating;

    /// Choose `n` from `pool` with repetition, as sorted index lists.
    fn combos(pool: &[usize], n: usize) -> Vec<Vec<usize>> {
        if n == 0 {
            return vec![vec![]];
        }
        let mut out = Vec::new();
        for (i, &p) in pool.iter().enumerate() {
            for mut rest in combos(&pool[i..], n - 1) {
                rest.push(p);
                out.push(rest);
            }
        }
        out
    }

    // The pool for each kind is capped to its strongest few. Without this the
    // weapon slot alone enumerates hundreds of thousands of combinations -
    // handles times damaging-with-repetition times accessories-with-repetition
    // - and every one of them costs a linear scan of the catalogue to rate.
    // The best of each kind is what any of this is looking for anyway.
    // Nine, not six. Six was cheap enough to run in five seconds and wrong
    // often enough to be worse than useless: it reported the Oracle dead when
    // it was reachable, then reported Templar, Warpriest, Juggernaut and
    // Spellblade dead the moment the pool pieces were priced properly and
    // displaced the physical ones out of the top six. Nine costs twenty
    // seconds and reports every class reachable, which is the truth.
    const POOL_CAP: usize = 9;
    let mut per_kind: Vec<Vec<Vec<usize>>> = Vec::new();
    for &(kind, min, max) in recipe {
        let mut pool: Vec<usize> = (0..CATALOG.len())
            // `fits`, not `slot ==`: materials are shared between gloves and
            // greaves and plating between helmets and greaves, so keying on
            // the home slot hid 22 of the 46 pieces a greave can take.
            .filter(|&i| CATALOG[i].fits(slot) && CATALOG[i].kind == kind)
            // Boss gear belongs to one creature. Without this the tool handed
            // the Money Jacket to every monster it authored.
            .filter(|&i| !gearmaster_engine::piece::is_boss_only(CATALOG[i].name))
            // A disconnected shape can never be part of an assembled item:
            // its islands flood-fill into groups of their own.
            .filter(|&i| connected(CATALOG[i].cells))
            .collect();
        pool.sort_by_key(|&i| std::cmp::Reverse(piece_rating(&CATALOG[i])));
        // Rating alone is not enough to choose a pool by. A piece can be worth
        // little on its own and be the entire point of a build - a spell that
        // answers its siblings rates poorly, because the rating cannot see the
        // ball it will sit in, so the six best spells never included one and
        // Oracle came back unreachable when it is reachable by hand.
        //
        // So: the best few by rating, then a few more that do something none
        // of those do.
        let mut kept: Vec<usize> = pool.iter().copied().take(POOL_CAP).collect();
        let shape = |i: usize| -> Vec<std::mem::Discriminant<Trigger>> {
            let mut v: Vec<_> = CATALOG[i].triggers.iter().map(std::mem::discriminant).collect();
            v.dedup();
            v
        };
        let mut seen: Vec<std::mem::Discriminant<Trigger>> =
            kept.iter().flat_map(|&i| shape(i)).collect();
        for &i in pool.iter().skip(POOL_CAP) {
            if kept.len() >= POOL_CAP + 4 {
                break;
            }
            let s = shape(i);
            if !s.is_empty() && s.iter().any(|d| !seen.contains(d)) {
                seen.extend(s);
                kept.push(i);
            }
        }
        pool = kept;
        let mut choices = Vec::new();
        for n in min..=max {
            choices.extend(combos(&pool, n));
        }
        per_kind.push(choices);
    }

    // Cartesian product across the kinds.
    let mut sets: Vec<Vec<usize>> = vec![vec![]];
    for choices in &per_kind {
        let mut next = Vec::new();
        for base in &sets {
            for c in choices {
                let mut v = base.clone();
                v.extend(c.iter().copied());
                next.push(v);
            }
        }
        sets = next;
    }

    let mut out: Vec<(i32, Vec<&'static str>)> = sets
        .into_iter()
        .filter(|s| s.iter().map(|&i| CATALOG[i].cells.len()).sum::<usize>() <= CELLS)
        .map(|s| {
            (
                s.iter().map(|&i| piece_rating(&CATALOG[i])).sum(),
                s.iter().map(|&i| CATALOG[i].name).collect(),
            )
        })
        .collect();
    out.sort_by_key(|(r, _)| std::cmp::Reverse(*r));
    out
}

/// Are a shape's cells one orthogonally connected blob? A piece that is not
/// contributes several groups, and a group missing the rest of its recipe
/// never assembles.
fn connected(cells: &[(i8, i8)]) -> bool {
    if cells.is_empty() {
        return false;
    }
    let mut seen = vec![cells[0]];
    let mut queue = vec![cells[0]];
    while let Some((x, y)) = queue.pop() {
        for n in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if cells.contains(&n) && !seen.contains(&n) {
                seen.push(n);
                queue.push(n);
            }
        }
    }
    seen.len() == cells.len()
}

/// The best-rated loadout for `slot` that actually packs and assembles.
/// `require_full` insists every cell be covered.
fn best_for(slot: SlotKind, require_full: bool) -> Option<(i32, Vec<(&'static str, u8, u8, u8)>)> {
    for (rating, names) in candidates(slot) {
        if require_full {
            let used: usize =
                names.iter().map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len()).sum();
            if used != CELLS {
                continue;
            }
        }
        if let Some(p) = pack(slot, &names) {
            return Some((rating, p));
        }
    }
    None
}

fn report(label: &str, slot: SlotKind, require_full: bool) {
    match best_for(slot, require_full) {
        None => println!("// {} {}: nothing packs", label, slot.name()),
        Some((rating, p)) => {
            let used: usize = p
                .iter()
                .map(|(n, ..)| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len())
                .sum();
            println!("// {} {} - rating {}, {} of {} cells", label, slot.name(), rating, used, CELLS);
            for (n, x, y, r) in p {
                println!("            (\"{}\", SlotKind::{:?}, {}, {}, {}),", n, slot, x, y, r);
            }
        }
    }
}

#[test]
#[ignore]
fn author_the_final_boss() {
    println!("\n===== FINAL BOSS: best that packs, every slot =====");
    for slot in SlotKind::ALL {
        report("final", slot, false);
    }
}

#[test]
#[ignore]
fn author_the_mid_boss_chest() {
    // Every one of the 48 cells covered, which takes several chestpieces:
    // one item holds a base and at most three layers.
    //   20 = Padded Base + Aegis Weave + Ironbark Layer
    //   16 = Padded Base + Plate Layer
    //   12 = Hide Base   + Hollow Weave  (+ whatever the split hands it)
    println!("\n===== MID BOSS: chest, every cell covered =====");
    let names = [
        "Padded Base",
        "Padded Base",
        "Hide Base",
        "Aegis Weave",
        "Ironbark Layer",
        "Plate Layer",
        "Hollow Weave",
    ];
    let used: usize = names
        .iter()
        .map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len())
        .sum();
    println!("// {} cells of {}", used, CELLS);
    emit("mid", SlotKind::Chest, &names);
}


#[test]
#[ignore]
fn author_the_mid_boss_rest() {
    println!("\n===== MID BOSS: one weapon, one glove, one greaves, two helmets =====");
    emit("mid", SlotKind::Weapon, &["Executioner's Haft", "Iron Blade", "Whetstone"]);
    emit("mid", SlotKind::Gloves, &["Leather Material", "Gripping Mold"]);
    emit("mid", SlotKind::Greaves, &["Runed Material", "Greave Mold"]);
    // Two separate helmets: two frames means two cores, so the split gives
    // two items even though they sit in one blob.
    emit(
        "mid",
        SlotKind::Helmet,
        &["Bone Frame", "Iron Plating", "Steel Frame", "Warding Plate"],
    );
}

/// Candidate layouts holding `k` finished items in one slot.
///
/// The single-item lists are what `candidates` produces, and a sampler built
/// from those can never exercise adjacency: two items never share a grid, so
/// the weave axis reads zero however it is weighted. Combining two of them and
/// packing the result gives the thing players actually build.
fn combined_candidates(slot: SlotKind, k: usize) -> Vec<(i32, Vec<&'static str>)> {
    let singles: Vec<(i32, Vec<&'static str>)> = candidates(slot).into_iter().take(120).collect();
    if k <= 1 {
        return singles;
    }
    let cells = |names: &[&'static str]| -> usize {
        names.iter().map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len()).sum()
    };
    let mut out = Vec::new();
    for (i, (ra, a)) in singles.iter().enumerate() {
        for (rb, b) in singles.iter().skip(i) {
            let mut names = a.clone();
            names.extend(b.iter().copied());
            if cells(&names) > CELLS {
                continue;
            }
            out.push((ra + rb, names));
        }
    }
    out.sort_by_key(|(r, _)| std::cmp::Reverse(*r));
    out.truncate(4000);
    out
}

/// Packable loadouts for a slot, spread across the range of what the
/// catalogue can build rather than only the best of them.
///
/// `n` targets are spaced evenly between the weakest and strongest legal
/// item; for each, the nearest candidate that actually packs is taken. That
/// gives a difficulty ramp made of gear instead of a ramp made of stat
/// multipliers on the same gear.
fn ladder_for(slot: SlotKind, n: usize) -> Vec<(i32, Vec<(&'static str, u8, u8, u8)>)> {
    ladder_of(slot, n, 1)
}

fn ladder_of(
    slot: SlotKind,
    n: usize,
    items: usize,
) -> Vec<(i32, Vec<(&'static str, u8, u8, u8)>)> {
    let cands = combined_candidates(slot, items);
    if cands.is_empty() {
        return Vec::new();
    }
    let best = cands.first().map(|(r, _)| *r).unwrap_or(0);
    let worst = cands.last().map(|(r, _)| *r).unwrap_or(0);

    let mut out: Vec<(i32, Vec<(&'static str, u8, u8, u8)>)> = Vec::new();
    let mut used: Vec<Vec<&'static str>> = Vec::new();
    for i in 0..n {
        let target = worst + (best - worst) * i as i32 / (n.max(2) - 1) as i32;
        // Nearest by rating, skipping anything already handed out so two
        // monsters never wear the identical thing.
        let mut by_distance: Vec<&(i32, Vec<&'static str>)> = cands.iter().collect();
        by_distance.sort_by_key(|(r, _)| (r - target).abs());
        for (rating, names) in by_distance.into_iter().take(400) {
            if used.contains(names) {
                continue;
            }
            if let Some(p) = pack(slot, names) {
                used.push(names.clone());
                out.push((*rating, p));
                break;
            }
        }
    }
    out
}

#[test]
#[ignore]
fn author_the_deep_ladder() {
    // One gear block per monster past the Gearwright, climbing.
    const N: usize = 20;
    let per_slot: Vec<Vec<(i32, Vec<(&'static str, u8, u8, u8)>)>> =
        SlotKind::ALL.iter().map(|&s| ladder_for(s, N)).collect();

    for i in 0..N {
        let mut total = 0;
        let mut lines = Vec::new();
        for (si, _) in SlotKind::ALL.iter().enumerate() {
            let rung = &per_slot[si];
            if rung.is_empty() {
                continue;
            }
            let (rating, placed) = &rung[i.min(rung.len() - 1)];
            total += rating;
            for (n, x, y, r) in placed {
                lines.push(format!(
                    "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
                    n,
                    SlotKind::ALL[si],
                    x,
                    y,
                    r
                ));
            }
        }
        println!("MONSTER {} rating {}", i, total);
        for l in lines {
            println!("{}", l);
        }
        println!("ENDMONSTER");
    }
}

// ===================================================== class reachability
//
// The axis reference values in `Fingerprint::of` were set by eye before there
// was gear to move them. This works out, from the catalogue itself, which
// classes a real build can actually reach and which one swallows everything.

use gearmaster_engine::class::{classify, rank, Axis, CLASSES};
use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::run::Run;

/// Put a packed layout onto a run, honouring duplicates.
fn wear(run: &mut Run, slot: SlotKind, placed: &[(&'static str, u8, u8, u8)]) -> bool {
    for (name, x, y, rot) in placed {
        let id = run
            .owned
            .iter()
            .copied()
            .find(|&id| run.registry.def(id).name == *name && !run.is_equipped(id));
        let Some(id) = id else { return false };
        run.registry.set_rotation(id, *rot);
        if run.equip(id, slot, *x, *y).is_err() {
            return false;
        }
    }
    true
}

/// How much a set of components pushes on one axis, roughly - enough to steer
/// a greedy search without duplicating the fingerprint's own maths.
fn pull(names: &[&'static str], axis: Axis) -> f32 {
    let mut total = 0.0;
    for n in names {
        let d = CATALOG.iter().find(|c| c.name == *n).unwrap();
        let s = &d.base;
        total += match axis {
            Axis::Arcana => s.magic_damage as f32,
            Axis::Brutality => s.physical_damage as f32,
            Axis::Ward => (s.physical_resist + s.magic_resist + s.physical_harden + s.magic_harden) as f32,
            Axis::Puncture => (s.physical_pierce + s.magic_pierce) as f32,
            Axis::Attunement => s.mana as f32 * 4.0,
            Axis::Wrath => s.rage as f32 * 6.0,
            Axis::Devotion => s.faith as f32 * 6.0,
            Axis::Growth => s.nature as f32 * 6.0,
            Axis::Bulwark => s.armor as f32,
            Axis::Cadence => 1.0,
            Axis::Mass => d.cells.len() as f32,
            Axis::Weave => 1.0,
            Axis::Malice => d.triggers.len() as f32,
            Axis::Sorcery => {
                if matches!(d.kind, PieceKind::Book | PieceKind::Orb) { 30.0 } else { 0.0 }
            }
            Axis::Orbits => if d.kind == PieceKind::Orb { 40.0 } else { 0.0 },
            // Steer hard towards spells that answer their siblings: they are
            // the point of committing to a ball rather than a book.
            Axis::Answering => {
                if d.triggers.iter().any(|t| matches!(t, Trigger::OnOtherCast(_))) {
                    30.0
                } else {
                    0.0
                }
            }
            Axis::MagicIn(sl) => {
                if d.slot == sl {
                    (s.magic_damage + s.magic_resist + s.magic_pierce + s.magic_harden) as f32
                        + if matches!(d.kind, PieceKind::Spell | PieceKind::Ink) { 8.0 } else { 0.0 }
                } else {
                    0.0
                }
            }
            Axis::PhysicalIn(sl) => {
                if d.slot == sl {
                    (s.physical_damage + s.physical_resist + s.physical_pierce) as f32
                } else {
                    0.0
                }
            }
        };
    }
    total
}

/// The best build this catalogue can offer for one class: per slot, the
/// packable loadout that pushes hardest on whatever that class asks for.
fn build_toward(class: &'static gearmaster_engine::class::ClassDef) -> Run {
    let mut run = Run::with_all_pieces();
    for slot in SlotKind::ALL {
        // Rank by what this class wants, not by rating. Taking the top of a
        // rating-sorted list would only ever look at heavy martial gear and
        // would report every other class dead for reasons of its own making.
        // Deliberately the full single-item list, not `combined_candidates`:
        // that prunes to the top singles by rating before pairing them, which
        // is the same rating bias that once reported every non-martial class
        // dead. Here the ranking has to be by what the class wants.
        let mut scored: Vec<(f32, Vec<&'static str>)> = candidates(slot)
            .into_iter()
            // A run owns one of each component, so a layout that wants two of
            // something cannot actually be worn - only monsters get those.
            .filter(|(_, names)| {
                let mut seen: Vec<&str> = Vec::new();
                names.iter().all(|n| {
                    if seen.contains(n) {
                        false
                    } else {
                        seen.push(n);
                        true
                    }
                })
            })
            .map(|(rating, names)| {
                let mut score: f32 = class.requires.iter().map(|&(a, _)| pull(&names, a)).sum();
                if class.requires.is_empty() {
                    score = rating as f32 * 0.01;
                }
                (score, names)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Try to actually wear each candidate, not just to pack it. Materials
        // are shared between gloves and greaves and plating between helmets
        // and greaves, so a build chosen for one slot can name a piece the
        // slot before it is already wearing - the run owns one of each. Before
        // this, greaves lost that race against gloves every single time and
        // dropped out of the analysis entirely.
        let mut worn = false;
        for (_, names) in scored.into_iter().take(150) {
            let Some(placed) = pack(slot, &names) else { continue };
            if wear(&mut run, slot, &placed) {
                worn = true;
                break;
            }
            // Put back whatever went on before it failed.
            for (name, ..) in &placed {
                if let Some(id) = run
                    .owned
                    .iter()
                    .copied()
                    .find(|&id| run.registry.def(id).name == *name && run.is_equipped(id))
                {
                    let _ = run.unequip(id);
                }
            }
        }
        if !worn {
            println!("  (could not fit {} for {})", slot.name(), class.name);
        }
    }
    run
}

#[test]
#[ignore]
fn which_classes_are_reachable() {
    // A dungeon class is not meant to be reachable by building - you go and
    // get it - so listing it as dead would be reporting the design as a bug.
    println!("\n=== can a real build reach each class? ===");
    let mut dead = Vec::new();
    for class in CLASSES.iter().filter(|c| !gearmaster_engine::class::is_earned(c.name)) {
        let run = build_toward(class);
        let fp = run.fingerprint();
        let got = classify(&fp).name;
        let detail: Vec<String> = class
            .requires
            .iter()
            .map(|&(a, need)| {
                let have = fp.get(a);
                format!("{} {}/{}{}", a.name(), have, need, if have >= need { "" } else { "  <-- short" })
            })
            .collect();
        let reached = rank(&fp).into_iter().any(|m| m.eligible && m.class.name == class.name);
        if !reached {
            dead.push(class.name);
        }
        println!(
            "{:<14} {:<10} best build lands on {:<14} [{}]",
            class.name,
            if reached { "REACHABLE" } else { "DEAD" },
            got,
            detail.join(", ")
        );
    }
    println!("\nunreachable: {:?}", dead);
}

#[test]
#[ignore]
fn which_class_dominates() {
    // Sample builds across the whole rating range and see where they land.
    println!("\n=== what a spread of builds classifies as ===");
    // Two items a slot, so the sample actually contains gear packed against
    // gear - which is what adjacency, and therefore weave, is about.
    let ladders: Vec<Vec<(i32, Vec<(&'static str, u8, u8, u8)>)>> = SlotKind::ALL
        .iter()
        .map(|&s| {
            let two = ladder_of(s, 12, 2);
            if two.is_empty() { ladder_of(s, 12, 1) } else { two }
        })
        .collect();

    let mut tally: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut weaves: Vec<i32> = Vec::new();
    let mut n = 0;
    for a in 0..12 {
        for shift in 0..12 {
            let mut run = Run::with_all_pieces();
            for (si, &slot) in SlotKind::ALL.iter().enumerate() {
                let l = &ladders[si];
                if l.is_empty() {
                    continue;
                }
                let pick = (a + si * shift) % l.len();
                wear(&mut run, slot, &l[pick].1);
            }
            let fp = run.fingerprint();
            weaves.push(fp.get(Axis::Weave));
            *tally.entry(classify(&fp).name).or_insert(0) += 1;
            n += 1;
        }
    }
    let mut rows: Vec<(&str, usize)> = tally.into_iter().collect();
    rows.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (name, count) in &rows {
        println!("{:<14} {:>4}  ({:.0}%)", name, count, *count as f32 * 100.0 / n as f32);
    }
    weaves.sort_unstable();
    let pc = |p: f32| weaves[((weaves.len() - 1) as f32 * p) as usize];
    println!(
        "weave spread: min {} p25 {} p50 {} p75 {} p90 {} max {}",
        weaves[0], pc(0.25), pc(0.5), pc(0.75), pc(0.9), weaves[weaves.len() - 1]
    );
    println!("{} builds sampled", n);
}

// ================================================== the balancing solver
//
// Medium is meant to be the fight the game was built around, so the question
// this answers is: on Medium, how far up the ladder does each kind of player
// get? Not one idealised build - four, because the people who play this are
// not all the same person.

#[derive(Copy, Clone, Debug)]
enum Profile {
    /// Buys and places without much thought. Legal builds, chosen at random.
    RandomBuilder,
    /// Never quite gets a recipe right. Pieces go down turned any which way,
    /// so most of what they own never assembles.
    NonAssembler,
    /// Wants to get on with it. Takes whatever is cheap and fast and fights.
    Grinder,
    /// Optimises. Takes the best-rated thing that fits.
    Optimiser,
    /// Fills every grid until nothing else will go in, taking the best-rated
    /// item that still fits at each step.
    Packer,
    /// The same, but choosing by worth per cell rather than worth outright -
    /// which is what packing tightly actually rewards.
    ValuePacker,
    /// Fills every grid choosing by worth per second - the time axis, where
    /// `ValuePacker` is the space one. Tests whether a build of many fast
    /// small triggers beats a build of few strong slow ones.
    SpeedPacker,
}

impl Profile {
    const ALL: &'static [Profile] = &[
        Profile::RandomBuilder,
        Profile::NonAssembler,
        Profile::Grinder,
        Profile::Optimiser,
        Profile::Packer,
        Profile::ValuePacker,
        Profile::SpeedPacker,
    ];

    fn name(self) -> &'static str {
        match self {
            Profile::RandomBuilder => "random builder",
            Profile::NonAssembler => "non-assembler",
            Profile::Grinder => "grinder (fast)",
            Profile::Optimiser => "optimiser (best)",
            Profile::Packer => "packer (dense)",
            Profile::ValuePacker => "packer (per cell)",
            Profile::SpeedPacker => "packer (per sec)",
        }
    }
}

/// Fill a grid until nothing else will go in, **locking as it goes**.
///
/// The other profiles hand the packer a fixed shopping list and take whatever
/// layout it finds, which caps a slot at one or two items however much room is
/// left over. A player does not build that way - they keep dropping things in
/// until the grid is full.
///
/// Locking is what makes the difference between a tidy grid and a full one. An
/// unlocked item re-derives its recipe from scratch every time a neighbour
/// lands, because pieces join their *nearest core*; so a new item dropped
/// alongside an old one can steal its pieces and break both. Without locks the
/// only safe placements are ones with a moat around them, and a 48-cell grid
/// runs out of moats after two or three items. Locking an item freezes its
/// membership, so the next item may sit flush against it - which is how the
/// player's friends fit six items in a slot, and now how these profiles do.
fn pack_dense(
    slot: SlotKind,
    rank: impl Fn(&PieceDefRef) -> i32,
) -> Vec<Vec<(&'static str, u8, u8, u8)>> {
    let mut ranked: Vec<(i32, Vec<&'static str>)> = candidates(slot)
        .into_iter()
        .map(|(r, names)| {
            let cells: i32 = names
                .iter()
                .map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len() as i32)
                .sum();
            let cooldown_ms = candidate_cooldown(slot, &names);
            (rank(&PieceDefRef { rating: r, cells, cooldown_ms }), names)
        })
        .collect();
    // Rank order alone is not a pool you can pack out of. For the plain
    // `Packer` the top of that list is the biggest, best items, so the first
    // one lands and nothing else in the pool will fit beside it - the profile
    // ends up wearing exactly one item per slot and calling it dense. Merge in
    // the same list ordered by worth *per cell* so there is small filler to go
    // in the gaps, and keep the big-first order for what gets tried first.
    let by_cells = |names: &[&'static str]| -> i32 {
        names
            .iter()
            .map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len() as i32)
            .sum::<i32>()
            .max(1)
    };
    ranked.sort_by_key(|(r, _)| std::cmp::Reverse(*r));
    let mut pool: Vec<(i32, Vec<&'static str>)> = ranked.iter().take(140).cloned().collect();
    ranked.sort_by_key(|(r, n)| std::cmp::Reverse(*r * 100 / by_cells(n)));
    for c in ranked.into_iter().take(140) {
        if !pool.iter().any(|(_, n)| *n == c.1) {
            pool.push(c);
        }
    }
    // Big first, then whatever still fits.
    pool.sort_by_key(|(r, _)| std::cmp::Reverse(*r));
    let ranked = pool;

    let mut reg = PieceRegistry::new();
    let mut loadout = Loadout::new();
    let mut placed: Vec<Vec<(&'static str, u8, u8, u8)>> = Vec::new();
    let mut used: Vec<&'static str> = Vec::new();

    // Keep going until a whole pass adds nothing.
    loop {
        let mut added = false;
        'candidate: for (_, names) in &ranked {
            if names.iter().any(|n| used.contains(n)) {
                continue;
            }
            // Try to seat this whole item somewhere in what is left.
            let ids: Vec<PieceId> = names
                .iter()
                .map(|n| reg.alloc(CATALOG.iter().position(|c| c.name == *n).unwrap()))
                .collect();
            // `seat_one_item` only returns true when these pieces landed as one
            // assembled item, and everything already down is locked, so nothing
            // it does can break what came before.
            if seat_one_item(&mut reg, &mut loadout, slot, &ids) {
                let g = loadout.slot(slot);
                // One inner vec per item. The boundary cannot be recovered
                // later: a prefix of a five-piece spell is very often a legal
                // three-piece spell, so anything that re-derives the split by
                // watching for "assembled" locks in the wrong place and the
                // rest of the slot merges into one dead blob.
                let mut item: Vec<(&'static str, u8, u8, u8)> = Vec::new();
                for (i, &id) in ids.iter().enumerate() {
                    let (x, y) = g.anchor_of(id).unwrap();
                    item.push((names[i], x, y, reg.rotation(id)));
                }
                placed.push(item);
                // Freeze it before looking for the next one.
                lock_assembled_in(&mut loadout, &reg, slot);
                used.extend(names.iter().copied());
                added = true;
                break 'candidate;
            }
        }
        if !added {
            break;
        }
    }
    placed
}

/// Seat every id somewhere such that everything in the slot still assembles.
///
/// The assembly check has to happen here, at the leaf, rather than on the
/// finished seating. Pieces join their nearest core, so dropping a new item
/// against an existing one can pull its pieces into that item's group and
/// break both recipes - and the first seating that merely *fits* is very often
/// one of those. Checking afterwards throws the whole candidate away; checking
/// here backtracks to a placement further off in the grid that works. Gloves
/// felt this worst, having the most spare room to be wrong in.
fn seat(
    reg: &mut PieceRegistry,
    loadout: &mut Loadout,
    slot: SlotKind,
    ids: &[PieceId],
    i: usize,
) -> Option<Vec<(u8, u8, u8)>> {
    // Complete seatings tested. Backtracking over a near-empty grid is
    // enormous, and the good placement turns up early or not at all.
    const SEATINGS: u32 = 400;
    fn go(
        reg: &mut PieceRegistry,
        loadout: &mut Loadout,
        slot: SlotKind,
        ids: &[PieceId],
        i: usize,
        budget: &mut u32,
    ) -> Option<Vec<(u8, u8, u8)>> {
        if i == ids.len() {
            if *budget == 0 {
                return None;
            }
            *budget -= 1;
            return loadout
                .report(reg, slot)
                .items
                .iter()
                .all(|it| it.assembled)
                .then(Vec::new);
        }
        let id = ids[i];
        for rot in 0..4u8 {
            reg.set_rotation(id, rot);
            for y in 0..SLOT_H {
                for x in 0..SLOT_W {
                    if loadout.can_place(reg, id, slot, x, y).is_err() {
                        continue;
                    }
                    loadout.slot_mut(slot).place(reg, id, x, y);
                    if let Some(mut rest) = go(reg, loadout, slot, ids, i + 1, budget) {
                        rest.insert(0, (x, y, rot));
                        return Some(rest);
                    }
                    loadout.slot_mut(slot).remove(id);
                    if *budget == 0 {
                        return None;
                    }
                }
            }
        }
        None
    }
    let mut budget = SEATINGS;
    go(reg, loadout, slot, ids, i, &mut budget)
}

/// Pick a loadout for one slot the way this profile would.
fn cached_candidates(slot: SlotKind) -> &'static [(i32, Vec<&'static str>)] {
    // Built once per slot: the pairing is expensive and the catalogue does
    // not change between runs.
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<Vec<(i32, Vec<&'static str>)>>> = OnceLock::new();
    let all = CACHE.get_or_init(|| {
        SlotKind::ALL
            .iter()
            .map(|&s| combined_candidates(s, 2).into_iter().take(600).collect())
            .collect()
    });
    &all[slot.index()]
}

fn choose(
    profile: Profile,
    slot: SlotKind,
    seed: u64,
) -> Option<Vec<Vec<(&'static str, u8, u8, u8)>>> {
    use gearmaster_engine::rating::piece_rating;
    match profile {
        Profile::Packer => return Some(pack_dense(slot, |d| d.rating)),
        // Worth per cell, scaled up so the division still ranks usefully.
        Profile::ValuePacker => {
            return Some(pack_dense(slot, |d| d.rating * 100 / d.cells.max(1)))
        }
        // Worth per second, not per cell. These two were once the same
        // function with a different scale factor on it, so they sorted
        // identically and the run learned nothing from having both.
        Profile::SpeedPacker => {
            return Some(pack_dense(slot, |d| d.rating * 1000 / d.cooldown_ms.max(1)))
        }
        _ => {}
    }
    let mut cands: Vec<(i32, Vec<&'static str>)> = cached_candidates(slot).to_vec();
    if cands.is_empty() {
        return None;
    }
    match profile {
        // The dense profiles never reach here; they returned above.
        Profile::Optimiser | Profile::Packer | Profile::ValuePacker | Profile::SpeedPacker => {}
        Profile::Grinder => {
            // Fast and cheap: sort by how often it would go off, not by worth.
            cands.sort_by_key(|(_, names)| {
                let cd: i32 = names
                    .iter()
                    .map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cooldown_ms as i32)
                    .filter(|c| *c > 0)
                    .min()
                    .unwrap_or(3000);
                (cd, names.iter().map(|n| {
                    CATALOG.iter().find(|c| c.name == *n).map(piece_rating).unwrap_or(0)
                }).sum::<i32>())
            });
        }
        Profile::RandomBuilder | Profile::NonAssembler => {
            let pick = (seed as usize * 2654435761) % cands.len();
            cands.swap(0, pick);
        }
    }
    for (_, names) in cands.into_iter().take(12) {
        if let Some(p) = pack(slot, &names) {
            // The non-assembler turns things. Most of it stops fitting its
            // recipe, which is the point of the profile.
            if matches!(profile, Profile::NonAssembler) {
                return Some(vec![p
                    .into_iter()
                    .enumerate()
                    .map(|(i, (n, x, y, _))| (n, x, y, ((seed as u8) + i as u8) % 4))
                    .collect()]);
            }
            return Some(vec![p]);
        }
    }
    None
}

/// Rungs to test at, rather than walking all 33. A full walk meant thousands
/// of simulations; five spot checks answer the same question - how deep does
/// this kind of player get - in a fraction of the time.
const BREAKPOINTS: [usize; 5] = [0, 8, 16, 24, 32];

/// One profile's board, packed once.
type Layout = Vec<(SlotKind, Vec<Vec<(&'static str, u8, u8, u8)>>)>;

/// Packing a 6x8 grid with backtracking is the expensive part, and a profile's
/// board does not change with the difficulty it is thrown at - only the fights
/// do. So every board is built once here and then reused across every setting.
fn all_layouts(seeds: &[u64]) -> Vec<Vec<Layout>> {
    Profile::ALL
        .iter()
        .map(|&profile| {
            seeds
                .iter()
                .map(|&seed| {
                    let mut out: Layout = Vec::new();
                    for slot in SlotKind::ALL {
                        if let Some(items) = choose(profile, slot, seed + slot.index() as u64) {
                            out.push((slot, items));
                        }
                    }
                    out
                })
                .collect()
        })
        .collect()
}

/// Health on both sides when the fight ended.
///
/// `CombatLog::player` and `::enemy` are the combatants as they *started* -
/// the interface uses them to lay the two boards out side by side - so reading
/// health off them tells you the pre-fight number and nothing else. A build
/// that lost at rung 41 still reports full health there. The end state has to
/// come from the events.
fn final_health(log: &gearmaster_engine::combat::CombatLog) -> (i32, i32) {
    use gearmaster_engine::combat::{Event, Side};
    let mut player = log.player.health;
    let mut enemy = log.enemy().health;
    for e in &log.entries {
        match &e.event {
            Event::Hit { by, target_health, .. } => match by {
                Side::Player => enemy = *target_health,
                Side::Enemy => player = *target_health,
            },
            Event::Burn { side, health, .. } | Event::Regen { side, health, .. } => match side {
                Side::Player => player = *health,
                Side::Enemy => enemy = *health,
            },
            Event::Fell { side } => match side {
                Side::Player => player = 0,
                Side::Enemy => enemy = 0,
            },
            _ => {}
        }
    }
    (player, enemy)
}

/// Put a prepared board on, **locking each item as it lands**.
///
/// This has to mirror `pack_dense` exactly or the numbers are fiction. The
/// packer seats an item, locks it, and seats the next one flush against it;
/// replaying that without the locks lets the core-anchoring re-derive from
/// scratch, and the tightly-packed board collapses into a couple of merged
/// blobs that assemble into nothing. A ten-piece weapon slot reading as two
/// items was this, not a balance result.
///
/// The layout carries the packer's own item boundaries, so this only has to
/// place each item's pieces and lock before starting the next one.
fn wear_layout(layout: &Layout) -> gearmaster_engine::run::Run {
    use gearmaster_engine::run::Run;
    let mut run = Run::with_all_pieces();
    for (slot, items) in layout {
        for item in items {
            let mut landed: Vec<PieceId> = Vec::new();
            for (name, x, y, rot) in item {
                // Buy another if every copy is already worn. The shop restocks
                // and nothing stops a player owning two Lonely Platings, so a
                // recipe that wants a piece twice - or a second slot that wants
                // what the helmet took - is a real build, not an illegal one.
                // Dropping those pieces instead is what made a ten-piece weapon
                // arrive as three.
                let id = match run
                    .owned
                    .iter()
                    .copied()
                    .find(|&i| run.registry.def(i).name == *name && !run.is_equipped(i))
                {
                    Some(id) => id,
                    None => {
                        let Some(d) = CATALOG.iter().position(|c| c.name == *name) else {
                            continue;
                        };
                        let id = run.registry.alloc(d);
                        run.owned.push(id);
                        id
                    }
                };
                run.registry.set_rotation(id, *rot);
                // A turned piece may no longer fit. That is the non-assembler
                // profile working, not a failure.
                match run.equip(id, *slot, *x, *y) {
                    Ok(_) => landed.push(id),
                    Err(e) => {
                        if std::env::var("WEAR_TRACE").is_ok() {
                            println!(
                                "      {:?} {:<22} ({},{}) rot {} REFUSED {:?}",
                                slot, name, x, y, rot, e
                            );
                        }
                    }
                }
            }
            // Lock it if it came out whole, so the next item can sit flush
            // against it instead of stealing its pieces.
            let whole = landed.first().and_then(|&p| {
                run.report(*slot)
                    .items
                    .into_iter()
                    .find(|it| it.assembled && it.pieces.contains(&p))
            });
            if let Some(it) = whole {
                run.toggle_lock_item(it.pieces[0]);
            }
        }
    }
    run
}

/// Wear a prepared board and see which breakpoints it can beat.
fn play_layout(layout: &Layout, difficulty: Difficulty) -> Vec<bool> {
    use gearmaster_engine::combat::Outcome;
    use gearmaster_engine::run::Mode;
    let mut run = wear_layout(layout);
    run.difficulty = difficulty;
    run.mode = Mode::Grinder;
    BREAKPOINTS
        .iter()
        .map(|&rung| {
            run.rung = rung;
            let won = run.fight_next().outcome == Outcome::Victory;
            run.back_to_loadout();
            won
        })
        .collect()
}

/// What a prepared board is worth, so the report can say *why* a profile
/// stalls rather than only that it did.
fn layout_summary(layout: &Layout) -> (usize, i32) {
    let run = wear_layout(layout);
    let items = run.combat_items();
    let stats = run.player_stats();
    let dps: i64 = items.iter().map(|i| i.dps_milli(stats.strength)).sum();
    (items.len(), (dps / 1000) as i32)
}


#[test]
#[ignore]
fn balance_report() {
    use gearmaster_engine::combat::Difficulty;
    let seeds = [1u64, 29];
    let layouts = all_layouts(&seeds);

    println!("\n=== which rungs each kind of player can beat ===");
    println!("medium is the intended fight. a profile should clear the early");
    println!("breakpoints there and start failing somewhere in the middle.\n");
    print!("{:<18}{:<12}{:>7}{:>7}", "profile", "setting", "items", "dps");
    for r in BREAKPOINTS {
        print!("{:>7}", format!("r{}", r + 1));
    }
    println!();
    for (pi, &profile) in Profile::ALL.iter().enumerate() {
        for &d in Difficulty::ALL {
            let mut wins = vec![0u8; BREAKPOINTS.len()];
            let (mut items, mut dps) = (0usize, 0i32);
            for (si, _) in seeds.iter().enumerate() {
                let l = &layouts[pi][si];
                let (n, p) = layout_summary(l);
                items += n;
                dps += p;
                for (i, w) in play_layout(l, d).into_iter().enumerate() {
                    wins[i] += w as u8;
                }
            }
            print!(
                "{:<18}{:<12}{:>7}{:>7}",
                profile.name(),
                format!("{} {}", d.name(), d.label()),
                items / seeds.len(),
                dps / seeds.len() as i32
            );
            for n in &wins {
                print!("{:>7}", match n {
                    2 => "win",
                    1 => "split",
                    _ => "-",
                });
            }
            println!();
        }
        println!();
    }
}

/// What each profile actually manages to wear, slot by slot.
///
/// The headline number in the balance reports is one integer, which is not
/// enough to tell "this build is weak" from "this build is one item". Print
/// the boards themselves so an under-packed profile is visible as a packing
/// bug rather than read as a balance result.
#[test]
#[ignore]
fn show_profile_packing() {
    let seeds = [1u64];
    let layouts = all_layouts(&seeds);
    println!("\n=== what each profile wears ===");
    for (pi, &profile) in Profile::ALL.iter().enumerate() {
        let l = &layouts[pi][0];
        let (items, dps) = layout_summary(l);
        println!("\n{}  -  {} items, {} dps", profile.name(), items, dps);
        let run = wear_layout(l);
        for (slot, wanted) in l {
            let cells: usize = wanted
                .iter()
                .flatten()
                .map(|(n, ..)| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len())
                .sum();
            let pieces: usize = wanted.iter().map(|it| it.len()).sum();
            let rep = run.report(*slot);
            let built: Vec<String> = rep
                .items
                .iter()
                .map(|it| format!("{}{}", it.pieces.len(), if it.assembled { "p" } else { "X" }))
                .collect();
            let on_board: usize = rep.items.iter().map(|it| it.pieces.len()).sum();
            println!(
                "  {:<9} {:>2}/{} cells, {:>2} of {:>2} pieces landed, wanted {} items -> [{}]",
                format!("{:?}", slot),
                cells,
                CELLS,
                on_board,
                pieces,
                wanted.len(),
                built.join(" ")
            );
        }
    }
    println!();
}

/// Walk each profile up the whole ladder at 1x and say where it stops.
///
/// `balance_report` samples five rungs, which answers "is the curve roughly
/// right" but not "how far does this build actually get" - the question you
/// ask when you want to know whether the game ends too early or never ends.
/// This fights every rung in order and prints the first loss, the last win,
/// and how many of the 50 fell over.
#[test]
#[ignore]
fn how_far_each_profile_gets() {
    use gearmaster_engine::combat::{Difficulty, Outcome, LADDER};
    use gearmaster_engine::run::Mode;

    let seeds = [1u64, 29, 77];
    let layouts = all_layouts(&seeds);

    println!("\n=== how far each profile gets at 1x (medium) ===");
    println!("every rung fought fresh, in order. 'first loss' is the wall;");
    println!("'last win' past it means the wall is a spike, not a ceiling.\n");
    print!(
        "{:<18}{:>5}{:>7}{:>7}{:>11}{:>10}{:>8}",
        "profile", "seed", "items", "dps", "first loss", "last win", "won"
    );
    println!("   losses up to the wall");

    for (pi, &profile) in Profile::ALL.iter().enumerate() {
        for (si, &seed) in seeds.iter().enumerate() {
            let l = &layouts[pi][si];
            let (items, dps) = layout_summary(l);

            let mut run = wear_layout(l);
            run.difficulty = Difficulty::Medium;
            run.mode = Mode::Grinder;

            let mut lost: Vec<usize> = Vec::new();
            let mut last_win: Option<usize> = None;
            for rung in 0..LADDER.len() {
                run.rung = rung;
                let won = run.fight_next().outcome == Outcome::Victory;
                run.back_to_loadout();
                if won {
                    last_win = Some(rung);
                } else {
                    lost.push(rung);
                }
            }
            let first_loss =
                lost.first().map(|r| format!("r{}", r + 1)).unwrap_or_else(|| "never".into());
            let last_win =
                last_win.map(|r| format!("r{}", r + 1)).unwrap_or_else(|| "none".into());
            let names: Vec<String> = lost
                .iter()
                .take(8)
                .map(|&r| format!("r{} {}", r + 1, LADDER[r].name))
                .collect();
            println!(
                "{:<18}{:>5}{:>7}{:>7}{:>11}{:>10}{:>8}   {}{}",
                profile.name(),
                seed,
                items,
                dps,
                first_loss,
                last_win,
                LADDER.len() - lost.len(),
                names.join(", "),
                if lost.len() > 8 { ", ..." } else { "" },
            );
        }
    }
    println!();
}

#[test]
#[ignore]
fn show_gear_by_difficulty() {
    use gearmaster_engine::combat::{Difficulty, LADDER};
    let wall = ["Warded Idol", "Mirror Fiend", "The Hollow King"];
    for spec in LADDER.iter().filter(|m| wall.contains(&m.name)) {
        println!("\n{}", spec.name);
        for &d in Difficulty::ALL {
            let names: Vec<&str> = spec.gear_at(d).iter().map(|g| g.0).collect();
            println!("  {:<8} {}", d.name(), names.join(", "));
        }
        {
            let written: Vec<&str> = spec.gear.iter().map(|g| g.0).collect();
            println!("  {:<8} {}", "written", written.join(", "));
        }
    }
}

/// What `pack_dense` ranks a candidate on.
pub struct PieceDefRef {
    pub rating: i32,
    pub cells: i32,
    /// How often the assembled item fires, in ms between triggers. Computed the
    /// same way `Loadout::report` does it: the core piece's cooldown, or its
    /// kind's default, divided by the speed the whole set adds up to.
    pub cooldown_ms: i32,
}

/// The cadence a candidate would assemble at.
///
/// Mirrors the cooldown arithmetic in `Loadout::report`. It has to be
/// recomputed here rather than read off a built item because ranking happens
/// before anything is placed.
fn candidate_cooldown(slot: SlotKind, names: &[&'static str]) -> i32 {
    use gearmaster_engine::curse::TICK_MS;
    use gearmaster_engine::piece::default_cooldown_ms;

    let defs = || names.iter().filter_map(|n| CATALOG.iter().find(|c| c.name == *n));
    let base = defs()
        .find(|d| d.kind.is_core())
        .map(|d| if d.cooldown_ms == 0 { default_cooldown_ms(slot) } else { d.cooldown_ms })
        .unwrap_or_else(|| default_cooldown_ms(slot)) as i32;
    let speed = (100 + defs().map(|d| d.speed_bonus).sum::<i32>()).max(10);
    (base * 100 / speed).max(TICK_MS as i32)
}

#[test]
#[ignore]
fn how_dense_is_dense() {
    for slot in SlotKind::ALL {
        let by_worth = pack_dense(slot, |d| d.rating);
        let by_cell = pack_dense(slot, |d| d.rating * 100 / d.cells.max(1));
        let by_sec = pack_dense(slot, |d| d.rating * 1000 / d.cooldown_ms.max(1));
        let cells = |p: &[Vec<(&'static str, u8, u8, u8)>]| -> usize {
            p.iter()
                .flatten()
                .map(|(n, ..)| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len())
                .sum()
        };
        let names = |p: &[Vec<(&'static str, u8, u8, u8)>]| -> Vec<&str> {
            let mut v: Vec<&str> = p.iter().flatten().map(|(n, ..)| *n).collect();
            v.sort();
            v
        };
        println!(
            "{:<11} worth {:>2}p/{:>2}c   per cell {:>2}p/{:>2}c   per sec {:>2}p/{:>2}c   \
             cell==sec: {}",
            slot.name(),
            by_worth.len(),
            cells(&by_worth),
            by_cell.len(),
            cells(&by_cell),
            by_sec.len(),
            cells(&by_sec),
            names(&by_cell) == names(&by_sec)
        );
    }
}

/// One gear block per monster above the old final boss, climbing. Francis is
/// authored separately: his chestpiece is built round a piece nobody else has.
#[test]
#[ignore]
fn author_the_summit() {
    const N: usize = 15;
    let per_slot: Vec<Vec<(i32, Vec<(&'static str, u8, u8, u8)>)>> =
        SlotKind::ALL.iter().map(|&s| ladder_for(s, N)).collect();

    for i in 0..N {
        let mut total = 0;
        let mut lines = Vec::new();
        for (si, _) in SlotKind::ALL.iter().enumerate() {
            let rung = &per_slot[si];
            if rung.is_empty() {
                continue;
            }
            // Take from the top of each slot's ladder: these sit above
            // everything already on it.
            let at = (rung.len() - 1).saturating_sub(N - 1 - i);
            let (rating, placed) = &rung[at];
            total += rating;
            for (n, x, y, r) in placed {
                lines.push(format!(
                    "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
                    n,
                    SlotKind::ALL[si],
                    x,
                    y,
                    r
                ));
            }
        }
        println!("// summit {} - gear rating {}", i, total);
        println!("        gear: &[");
        for l in &lines {
            println!("{}", l);
        }
        println!("        ],");
    }
}

// ---------------------------------------------------------------------------
// Authoring the named fights, with locking.
//
// The old seater packed a slot and then asked "did everything assemble?".
// That question gets harder the more you put in, because an unlocked board
// negotiates with itself: the optional pieces drift to whichever core is
// nearest, so a second item packed flush against the first can quietly steal
// from it. The tool answered by leaving room, which is why every boss on the
// ladder was wearing exactly one item per slot.
//
// Locking is the way out, and it is the same button the player has. Seat one
// item, lock it, seat the next against it. A locked item cannot be joined and
// cannot lose a piece, so "flush" stops being dangerous and a 6x8 grid turns
// out to hold three of them.

/// Seat one item's pieces into a slot that may already hold locked ones.
///
/// Succeeds only if every piece lands *and* the pieces just placed come out as
/// a single assembled item of their own.
fn seat_one_item(
    reg: &mut PieceRegistry,
    loadout: &mut Loadout,
    slot: SlotKind,
    ids: &[PieceId],
) -> bool {
    fn go(
        reg: &mut PieceRegistry,
        loadout: &mut Loadout,
        slot: SlotKind,
        ids: &[PieceId],
        i: usize,
        budget: &mut u32,
    ) -> bool {
        if i == ids.len() {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            // The new pieces must be one assembled item, and the items that
            // were already there must still be assembled - a locked one always
            // is, which is the whole point.
            let rep = loadout.report(reg, slot);
            let mine = rep
                .items
                .iter()
                .find(|it| it.pieces.iter().any(|p| ids.contains(p)));
            return match mine {
                Some(it) => it.assembled && ids.iter().all(|p| it.pieces.contains(p)),
                None => false,
            };
        }
        let id = ids[i];
        for rot in Packer::distinct_rotations(reg, id) {
            reg.set_rotation(id, rot);
            for y in 0..SLOT_H {
                for x in 0..SLOT_W {
                    if loadout.can_place(reg, id, slot, x, y).is_err() {
                        continue;
                    }
                    // An item is one connected blob, so every piece after the
                    // first has to touch one already down. Without this the
                    // search wanders the whole 48-cell grid for every piece
                    // and never finishes.
                    if i > 0 {
                        let touches = {
                            let g = loadout.slot(slot);
                            let mut ok = false;
                            for &(dx, dy) in reg.shape(id).cells() {
                                let (cx, cy) = (x as i32 + dx as i32, y as i32 + dy as i32);
                                for (ax, ay) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                                    let (nx, ny) = (cx + ax, cy + ay);
                                    if nx < 0 || ny < 0 || nx >= SLOT_W as i32 || ny >= SLOT_H as i32
                                    {
                                        continue;
                                    }
                                    if let Some(other) = g.get(nx as u8, ny as u8) {
                                        if ids[..i].contains(&other) {
                                            ok = true;
                                        }
                                    }
                                }
                            }
                            ok
                        };
                        if !touches {
                            continue;
                        }
                    }
                    loadout.slot_mut(slot).place(reg, id, x, y);
                    if go(reg, loadout, slot, ids, i + 1, budget) {
                        return true;
                    }
                    loadout.slot_mut(slot).remove(id);
                    if *budget == 0 {
                        return false;
                    }
                }
            }
        }
        false
    }
    let mut budget = 1200u32;
    go(reg, loadout, slot, ids, 0, &mut budget)
}

/// One item's worth of pieces, per candidate.
///
/// Not `cached_candidates`, which hands back *two* items concatenated - that
/// pool exists for the old seater, which packed a whole slot at once. Seating
/// one item at a time needs one item at a time.
fn cached_singles(slot: SlotKind) -> &'static [(i32, Vec<&'static str>)] {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<Vec<(i32, Vec<&'static str>)>>> = OnceLock::new();
    let all = CACHE.get_or_init(|| {
        SlotKind::ALL
            .iter()
            .map(|&s| {
                let cells = |names: &[&'static str]| -> usize {
                    names
                        .iter()
                        .map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len())
                        .sum()
                };
                // Two orderings, merged. Rating alone is not a pool you can
                // pack three items out of: the best chestpieces are the
                // biggest, so the top 300 by rating had nothing under twelve
                // cells in it and a boss could never wear three of them. The
                // second half is by worth per cell, which is what "compact"
                // means when you are trying to fit more than one.
                let full = candidates(s);
                let mut out: Vec<(i32, Vec<&'static str>)> =
                    full.iter().take(200).cloned().collect();
                let mut dense = full;
                dense.sort_by_key(|(r, n)| std::cmp::Reverse(*r * 100 / cells(n).max(1) as i32));
                for c in dense.into_iter().take(200) {
                    if !out.iter().any(|(_, n)| *n == c.1) {
                        out.push(c);
                    }
                }
                // And the weak end. A pool made only of the best and the most
                // compact has no floor: the first named board authored against
                // a rung-nine target came out eight times what the creatures
                // either side of it were wearing, because nothing weaker than
                // fifty marks existed to pick.
                let mut weak = candidates(s);
                weak.sort_by_key(|(r, _)| *r);
                for c in weak.into_iter().take(200) {
                    if !out.iter().any(|(_, n)| *n == c.1) {
                        out.push(c);
                    }
                }
                out
            })
            .collect()
    });
    &all[slot.index()]
}

/// Pack `want` assembled items into one slot, locking each as it lands.
///
/// Returns the placements and what the whole slot rates.
fn pack_locked(
    slot: SlotKind,
    want: usize,
    seed: u64,
    target_per_item: i32,
    allow: fn(&'static gearmaster_engine::piece::PieceDef) -> bool,
) -> Option<(Vec<(&'static str, u8, u8, u8)>, i32, Vec<usize>)> {
    let mut reg = PieceRegistry::new();
    let mut loadout = Loadout::new();
    let mut out: Vec<(&'static str, u8, u8, u8)> = Vec::new();
    let mut sizes: Vec<usize> = Vec::new();

    // Aimed at a rating, not at the top of the catalogue.
    //
    // Picking the best available is what the tool did for the old one-item
    // boards, and it does not survive being asked for ten: a mini-boss at rung
    // nine came out rating 1954 against rung thirteen's 83, which is not
    // difficulty, it is a wall. The target comes from what the creatures
    // either side of it are wearing.
    // A creature can be a particular sort of creature. Without this the tool
    // packs whatever rates nearest the target, which gave a thing that was
    // supposed to only get into your head a weapon and seventeen strength.
    //
    // The filter runs over the *whole* candidate list, not the cached spread.
    // Filtering the cache gave zero weapons out of five hundred: the cache is
    // the top by rating, the top by density and the bottom, and every quiet
    // book-and-ink weapon sits in the middle where none of those three look.
    let owned: Vec<(i32, Vec<&'static str>)>;
    let pool: Vec<&(i32, Vec<&'static str>)> = if is_narrowed(allow) {
        owned = candidates(slot)
            .into_iter()
            .filter(|(_, names)| {
                names.iter().all(|n| CATALOG.iter().find(|d| d.name == *n).is_some_and(allow))
            })
            .collect();
        owned.iter().collect()
    } else {
        cached_singles(slot).iter().collect()
    };
    if pool.is_empty() {
        return None;
    }

    let mut rng = seed | 1;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    let cells_of = |names: &[&'static str]| -> usize {
        names.iter().map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len()).sum()
    };
    // How much room the ones still to come will need, so the first item does
    // not eat the whole grid. Three chest items would not fit at all until
    // this existed: the pool is sorted by rating, the best chestpieces are
    // the biggest, and the seater kept picking three of them.
    let mut placed_items = 0usize;
    let mut tries = 0usize;
    let mut used = 0usize;
    while placed_items < want && tries < 200 {
        tries += 1;
        let left = want - placed_items;
        let room = CELLS.saturating_sub(used);
        // Leave the ones after this at least four cells each.
        let cap = room.saturating_sub(4 * (left - 1));
        let mut fits: Vec<&&(i32, Vec<&'static str>)> =
            pool.iter().filter(|(_, n)| cells_of(n) <= cap).collect();
        if fits.is_empty() {
            break;
        }
        // Nearest the target first. The later items also have to fit around
        // what is already locked down, so they are broken toward the compact
        // end of the equally-good ones - a boss could never get three
        // chestpieces on until this existed, because the seater kept reaching
        // for three of the largest in the game.
        fits.sort_by_key(|(r, n)| {
            let miss = (*r - target_per_item).abs();
            if placed_items == 0 { (miss, 0usize) } else { (miss, cells_of(n)) }
        });
        let span = fits.len().min(22);
        let pick = fits[(next() as usize) % span.max(1)];
        let ids: Vec<PieceId> = pick
            .1
            .iter()
            .filter_map(|n| CATALOG.iter().position(|d| d.name == *n))
            .map(|i| reg.alloc(i))
            .collect();
        if ids.len() != pick.1.len() {
            continue;
        }
        if seat_one_item(&mut reg, &mut loadout, slot, &ids) {
            for (&id, &name) in ids.iter().zip(pick.1.iter()) {
                let (x, y) = loadout.slot(slot).anchor_of(id).unwrap();
                out.push((name, x, y, reg.rotation(id)));
            }
            gearmaster_engine::loadout::lock_assembled_in(&mut loadout, &reg, slot);
            sizes.push(ids.len());
            used += cells_of(&pick.1);
            placed_items += 1;
        } else {
            for id in ids {
                loadout.slot_mut(slot).remove(id);
            }
        }
    }
    if placed_items < want {
        return None;
    }
    let rating: i32 = loadout
        .report(&reg, slot)
        .items
        .iter()
        .filter(|it| it.assembled)
        .map(|it| it.rating)
        .sum();
    Some((out, rating, sizes))
}

/// Is this filter actually narrowing anything? A creature with no restriction
/// uses the cached spread, which is the fast path and the one nearly every
/// board takes.
fn is_narrowed(allow: fn(&'static gearmaster_engine::piece::PieceDef) -> bool) -> bool {
    CATALOG.iter().any(|d| !allow(d))
}

/// Where an alternate stands, so it can be targeted like anything else.
fn alternate_rung(name: &str) -> usize {
    use gearmaster_engine::event::{Outcome, EVENTS};
    if let Some(e) = EVENTS.iter().find(|e| {
        e.choices.iter().any(|c| matches!(c.outcome, Outcome::FightInstead(n) if n == name))
    }) {
        return e.at;
    }
    // A dungeon floor is pitched at the rung its door stands on, stepping up a
    // little with each floor down.
    for d in gearmaster_engine::dungeon::DUNGEONS {
        if let Some(i) = d.floors.iter().position(|f| f.creature == name) {
            return 9 + i * 2;
        }
    }
    0
}

/// Print dense boards for every boss and mini-boss, ready to paste.
#[test]
#[ignore]
fn author_the_named_fights() {
    use gearmaster_engine::combat::{Rank, LADDER};
    // What the creatures either side of a named one are wearing, so its board
    // is denser than theirs without being from a different game.
    let ordinary: Vec<(usize, i32)> = LADDER
        .iter()
        .enumerate()
        .filter(|(_, m)| m.rank == Rank::Ordinary)
        .map(|(i, m)| {
            let (reg, lo) = m.loadout();
            let r: i32 = SlotKind::ALL
                .iter()
                .flat_map(|s| {
                    lo.report(&reg, *s)
                        .items
                        .into_iter()
                        .filter(|it| it.assembled)
                        .map(|it| it.rating)
                        .collect::<Vec<_>>()
                })
                .sum();
            (i, r)
        })
        .collect();

    // Alternates too - they are named fights that happen to be off the road,
    // and hand-placing one is how the Dreaming Idiot ended up with a board
    // that assembled into nothing.
    let named: Vec<(usize, &'static gearmaster_engine::combat::MonsterSpec)> = LADDER
        .iter()
        .enumerate()
        .filter(|(_, m)| m.rank != Rank::Ordinary)
        .chain(
            gearmaster_engine::combat::ALTERNATES
                .iter()
                .map(|m| (alternate_rung(m.name), m)),
        )
        .collect();
    for (idx, m) in named {
        let want = m.rank.min_items_per_slot();
        // What this one is allowed to be made of.
        let allow: fn(&'static gearmaster_engine::piece::PieceDef) -> bool =
            if m.name == "The Dreaming Idiot" {
                // Mind and growth only: it never swings and it deals nothing
                // that can be healed back.
                |d| {
                    d.base.strength == 0
                        && d.base.physical_damage == 0
                        && d.base.magic_damage == 0
                        && d.base.rage == 0
                        && !format!("{:?}", d.triggers).contains("Damage {")
                }
            } else {
                |_| true
            };
        // The nearest ordinary rung on either side, averaged.
        let mut near: Vec<(usize, i32)> = ordinary.clone();
        near.sort_by_key(|(i, _)| (*i as i64 - idx as i64).abs());
        let base: i32 = near.iter().take(2).map(|(_, r)| *r).sum::<i32>() / 2;
        // A mini-boss is worth about two ordinary rungs, a boss about three.
        let step = if m.rank == Rank::Boss { 3.0 } else { 1.9 };
        let total_target = (base as f32 * step) as i32;
        let target_per_item = (total_target / 5 / want as i32).max(6);
        println!("\n// ---- {} ({:?}, {} items a slot, target {}) ----", m.name, m.rank, want, total_target);
        let mut total = 0;
        let mut chunks: Vec<usize> = Vec::new();
        println!("        gear: &[");
        for slot in SlotKind::ALL {
            let mut best: Option<(Vec<(&'static str, u8, u8, u8)>, i32, Vec<usize>)> = None;
            // As many as will go. A creature narrowed to one kind of harm may
            // simply not have three weapons in it - the Dreaming Idiot deals
            // nothing but mind damage, and every weapon recipe in the game
            // wants something that hits. One voice is the right answer there,
            // not a fourth helmet pretending to be a weapon.
            let mut want = want;
            while want > 1 && pack_locked(slot, want, 1, target_per_item, allow).is_none() {
                want -= 1;
            }
            for seed in 0..6u64 {
                let s = seed
                    .wrapping_mul(0x9E37_79B9)
                    .wrapping_add(m.name.bytes().map(|b| b as u64).sum::<u64>());
                if let Some(got) = pack_locked(slot, want, s, target_per_item, allow) {
                    let want_slot = total_target / 5;
                    let closer = best
                        .as_ref()
                        .is_none_or(|b| (got.1 - want_slot).abs() < (b.1 - want_slot).abs());
                    if closer {
                        best = Some(got);
                    }
                }
            }
            match best {
                Some((placed, r, sizes)) => {
                    total += r;
                    for (n, x, y, rot) in placed {
                        println!("            (\"{}\", SlotKind::{:?}, {}, {}, {}),", n, slot, x, y, rot);
                    }
                    chunks.extend(sizes);
                }
                None => println!("            // FAILED to pack {} items into {:?}", want, slot),
            }
        }
        println!("        ],  // total gear rating {}", total);
        println!("        items: &{:?},", chunks);
    }
}


#[test]
#[ignore]
fn author_the_idiots_weapon() {
    // One voice. A creature that deals nothing but mind damage has exactly one
    // weapon in it, and the orb-and-Unmaking build is the whole of what the
    // catalogue offers that does no other kind of harm.
    for names in [
        vec!["Grovemind Orb", "Siphon", "Siphon", "Siphon", "Rootwork Alignment"],
        vec!["Scrying Orb", "Unmaking", "Unmaking", "Verdant Alignment"],
        vec!["Pocket Grimoire", "Hollow Ink", "Siphon"],
    ] {
        match pack(SlotKind::Weapon, &names) {
            Some(p) => {
                println!("// {:?}", names);
                for (n, x, y, r) in &p {
                    println!("            (\"{}\", SlotKind::Weapon, {}, {}, {}),", n, x, y, r);
                }
                println!("            // items: {}", p.len());
            }
            None => println!("// {:?} does not pack", names),
        }
    }
}

/// One more assembled item for ten ordinary rungs, each answering something
/// their build already leans on.
///
/// Seated onto the board they already have, with everything on it locked
/// first, so the addition has to fit round the arrangement rather than
/// renegotiate it. Hand-placing these is how a board ends up assembling into
/// nothing.
#[test]
#[ignore]
fn author_the_extra_items() {
    use gearmaster_engine::combat::LADDER;
    use gearmaster_engine::piece::PieceDef;

    // (creature, slot to add to, what it should reinforce)
    let picks: &[(&str, SlotKind, fn(&&'static PieceDef) -> bool)] = &[
        ("Bog Toad", SlotKind::Chest, |d| d.base.health > 0 || d.base.armor > 0),
        ("Iron Sentinel", SlotKind::Helmet, |d| d.base.armor > 0 || d.base.physical_resist > 0),
        // Gloves rather than a second weapon: requiring every piece of a
        // weapon candidate to be mana or spell forces an orb build, and the
        // cheapest one that qualified rated 191 at rung twelve.
        ("Rust Colossus", SlotKind::Gloves, |d| d.base.mana > 0 || d.base.armor > 0),
        ("Grave Chorus", SlotKind::Gloves, |d| d.base.rage > 0 || d.base.physical_damage > 0),
        ("Cog Priest", SlotKind::Weapon, |d| d.base.physical_damage > 0 || d.base.strength > 0),
        ("Mire Behemoth", SlotKind::Chest, |d| d.base.armor > 0 || d.base.health > 0),
        ("Null Sentinel", SlotKind::Greaves, |d| d.base.armor > 0 || d.base.physical_resist > 0),
        ("Iron Abbot", SlotKind::Helmet, |d| d.base.faith > 0 || d.base.magic_resist > 0),
        ("The Quiet Hour", SlotKind::Chest, |d| d.base.armor > 0),
        ("Anvilheart", SlotKind::Helmet, |d| d.base.armor > 0 || d.base.physical_harden > 0),
    ];

    for (name, slot, want) in picks {
        let m = LADDER.iter().find(|m| m.name == *name).expect("on the ladder");
        let (mut reg, mut loadout) = m.loadout();
        // Lock what is there. The addition fits round it or not at all.
        for k in SlotKind::ALL {
            gearmaster_engine::loadout::lock_assembled_in(&mut loadout, &reg, k);
        }
        let before = loadout.report(&reg, *slot).items.iter().filter(|i| i.assembled).count();

        // Modest: one more item, not a second build. Aimed at a third of what
        // the creature already carries in that slot.
        let target = loadout
            .report(&reg, *slot)
            .items
            .iter()
            .filter(|i| i.assembled)
            .map(|i| i.rating)
            .max()
            .unwrap_or(30)
            / 2;

        let mut best: Option<(i32, Vec<(&'static str, u8, u8, u8)>)> = None;
        for cand in cached_singles(*slot) {
            if !cand.1.iter().all(|n| CATALOG.iter().find(|d| d.name == *n).is_some_and(|d| want(&d)))
            {
                continue;
            }
            if best.as_ref().is_some_and(|(r, _)| (cand.0 - target).abs() >= (*r - target).abs()) {
                continue;
            }
            let ids: Vec<PieceId> = cand
                .1
                .iter()
                .filter_map(|n| CATALOG.iter().position(|d| d.name == *n))
                .map(|i| reg.alloc(i))
                .collect();
            if seat_one_item(&mut reg, &mut loadout, *slot, &ids) {
                let placed: Vec<(&'static str, u8, u8, u8)> = ids
                    .iter()
                    .zip(cand.1.iter())
                    .map(|(&id, &n)| {
                        let (x, y) = loadout.slot(*slot).anchor_of(id).unwrap();
                        (n, x, y, reg.rotation(id))
                    })
                    .collect();
                best = Some((cand.0, placed));
                for id in &ids {
                    loadout.slot_mut(*slot).remove(*id);
                }
            } else {
                for id in &ids {
                    loadout.slot_mut(*slot).remove(*id);
                }
            }
        }
        match best {
            Some((r, placed)) => {
                println!("// {} : +1 {:?} (rating {}, was {} items)", name, slot, r, before);
                for (n, x, y, rot) in &placed {
                    println!("            (\"{}\", SlotKind::{:?}, {}, {}, {}),", n, slot, x, y, rot);
                }
                println!("// items +{}", placed.len());
            }
            None => println!("// {} : nothing fits in {:?}", name, slot),
        }
    }
}


/// What each item is actually worth in a fight, against what it costs.
///
/// The rating is a *model* of worth and the shop price is derived from it, so
/// nothing in the pricing path can tell you when the model is wrong. This
/// measures instead: swap one item into an otherwise fixed board, fight a
/// fixed opponent, and see what changed.
///
/// The board has to be fixed and complete. Measuring an item on a bare board
/// only measures survivability - a lone weapon dies before its first swing and
/// scores nothing, while any helmet racks up armour - which says everything
/// about the experiment and nothing about the gear.
#[test]
#[ignore]
fn find_outlier_gear() {
    use gearmaster_engine::combat::{Difficulty, Outcome, LADDER};
    use gearmaster_engine::rating::shop_price;
    use gearmaster_engine::run::Mode;

    // The rung the packed profiles clear but only just, so a better item can
    // show as a wider win and a worse one as a loss. Anything they crush or
    // cannot touch measures nothing.
    let target = LADDER[19];

    let baseline: Vec<(SlotKind, Vec<Vec<(&'static str, u8, u8, u8)>>)> =
        SlotKind::ALL.iter().map(|&s| (s, pack_dense(s, |d| d.rating))).collect();

    let run_board = |layout: &Layout| -> (bool, i32) {
        let mut run = wear_layout(layout);
        run.difficulty = Difficulty::Medium;
        run.mode = Mode::Grinder;
        let log = run.fight(&target);
        let won = log.outcome == Outcome::Victory;
        // One scalar for "how comfortably": what you had left minus what they
        // had left. Positive is a win with room, negative is a loss.
        let (ph, eh) = final_health(log);
        (won, ph - eh)
    };

    let base_layout: Layout = baseline.clone();
    let (base_won, base_margin) = run_board(&base_layout);

    struct Row {
        slot: SlotKind,
        names: Vec<&'static str>,
        price: i32,
        margin: i32,
        won: bool,
    }
    let mut rows: Vec<Row> = Vec::new();

    for slot in SlotKind::ALL {
        let mut all = candidates(slot);
        let cells = |names: &[&'static str]| -> i32 {
            names
                .iter()
                .map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len() as i32)
                .sum::<i32>()
                .max(1)
        };
        let mut sample: Vec<Vec<&'static str>> =
            all.iter().take(200).map(|(_, n)| n.clone()).collect();
        all.sort_by_key(|(r, n)| std::cmp::Reverse(*r * 100 / cells(n)));
        for (_, n) in all.into_iter().take(200) {
            if !sample.contains(&n) {
                sample.push(n);
            }
        }

        for names in sample {
            let Some(placed) = pack(slot, &names) else { continue };
            // Everything else stays exactly as the baseline had it.
            let layout: Layout = baseline
                .iter()
                .map(|(s, items)| {
                    if *s == slot { (*s, vec![placed.clone()]) } else { (*s, items.clone()) }
                })
                .collect();
            let (won, margin) = run_board(&layout);
            let price: i32 = names
                .iter()
                .map(|n| shop_price(CATALOG.iter().find(|c| c.name == *n).unwrap()))
                .sum();
            rows.push(Row { slot, names, price: price.max(1), margin, won });
        }
    }

    println!("\n=== what gear is worth against what it costs ===");
    println!("one item swapped into an otherwise fixed dense-packed board,");
    println!("fought against {} (rung 20) at 1x.", target.name);
    println!("margin = your health left minus theirs; the whole board without");
    println!("any swap scores {} ({}).", base_margin, if base_won { "win" } else { "loss" });
    println!("{} items measured.\n", rows.len());

    let show = |label: &str, rows: &[&Row]| {
        println!("\n{}", label);
        println!("{:<11}{:>7}{:>9}{:>7}   {}", "slot", "price", "margin", "won", "item");
        for r in rows {
            println!(
                "{:<11}{:>7}{:>9}{:>7}   {}",
                r.slot.name(),
                r.price,
                r.margin,
                if r.won { "yes" } else { "-" },
                r.names.join(" + ")
            );
        }
    };

    for slot in SlotKind::ALL {
        let mut v: Vec<&Row> = rows.iter().filter(|r| r.slot == slot).collect();
        if v.is_empty() {
            continue;
        }
        v.sort_by_key(|r| std::cmp::Reverse(r.margin));
        let wins = v.iter().filter(|r| r.won).count();
        println!(
            "\n---- {} : {} measured, {} of them win, best {} worst {} ----",
            slot.name(),
            v.len(),
            wins,
            v[0].margin,
            v[v.len() - 1].margin
        );
        let best: Vec<&Row> = v.iter().take(8).copied().collect();
        show("strongest", &best);
        // Cheap and still winning is the real outlier: it is what a player
        // finds by accident and then never takes off.
        let mut cheap: Vec<&Row> = v.iter().filter(|r| r.won && r.price <= 120).copied().collect();
        cheap.sort_by_key(|r| std::cmp::Reverse(r.margin * 100 / r.price.max(1)));
        if !cheap.is_empty() {
            show("best value under 120g", &cheap.into_iter().take(8).collect::<Vec<_>>());
        }
        let mut dear: Vec<&Row> = v.iter().filter(|r| !r.won && r.price >= 250).copied().collect();
        dear.sort_by_key(|r| r.margin);
        if !dear.is_empty() {
            show("expensive and still losing", &dear.into_iter().take(8).collect::<Vec<_>>());
        }
    }

    // Which components keep turning up at the top of their slot.
    let mut tally: Vec<(&'static str, usize)> = Vec::new();
    for slot in SlotKind::ALL {
        let mut v: Vec<&Row> = rows.iter().filter(|r| r.slot == slot).collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.margin));
        for r in v.iter().take(25) {
            for n in &r.names {
                match tally.iter_mut().find(|(m, _)| m == n) {
                    Some((_, c)) => *c += 1,
                    None => tally.push((n, 1)),
                }
            }
        }
    }
    tally.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("\ncomponents appearing most often in the best 25 of their slot:");
    for (n, c) in tally.iter().take(25) {
        println!("  {:>3}x  {}", c, n);
    }
    println!();
}

/// The shape of the ladder: how comfortably one fixed board takes each rung.
///
/// Win/loss alone cannot tell a wall from a cliff. A rung that is lost by two
/// points is tuned; one lost by four thousand is a different game starting
/// without warning. Printing the margin per rung shows which is which, and
/// where the steps are.
#[test]
#[ignore]
fn ladder_curve() {
    use gearmaster_engine::combat::{Difficulty, Outcome, LADDER};
    use gearmaster_engine::run::Mode;

    let boards: Vec<(&str, Layout)> = vec![
        ("dense", SlotKind::ALL.iter().map(|&s| (s, pack_dense(s, |d| d.rating))).collect()),
        (
            "per sec",
            SlotKind::ALL
                .iter()
                .map(|&s| (s, pack_dense(s, |d| d.rating * 1000 / d.cooldown_ms.max(1))))
                .collect(),
        ),
    ];

    println!("\n=== how the ladder feels to a packed board ===");
    println!("margin = your health left minus theirs, at 1x.");
    println!("a big negative jump between neighbours is a cliff, not a curve.\n");
    print!("{:<4}{:<22}{:>8}", "rung", "monster", "health");
    for (name, _) in &boards {
        print!("{:>12}", *name);
    }
    println!();

    let mut prev: Vec<i32> = vec![0; boards.len()];
    for (i, spec) in LADDER.iter().enumerate() {
        print!("{:<4}{:<22}{:>8}", i + 1, spec.name, spec.health);
        for (bi, (_, layout)) in boards.iter().enumerate() {
            let mut run = wear_layout(layout);
            run.difficulty = Difficulty::Medium;
            run.mode = Mode::Grinder;
            run.rung = i;
            let log = run.fight_next();
            let (ph, eh) = final_health(log);
            let m = ph - eh;
            // A stalemate is the 60-second clock, and it costs a run life
            // exactly like dying does - so a fight can be lost from thousands
            // of health ahead. Worth telling apart from being killed.
            let mark = match log.outcome {
                Outcome::Victory => " ",
                Outcome::Defeat => "D",
                Outcome::Stalemate => "S",
            };
            let step = m - prev[bi];
            prev[bi] = m;
            print!("{:>12}", format!("{}{}{}", m, mark, if step < -1500 { "!" } else { "" }));
        }
        println!();
    }
        println!("\nD = killed, S = ran out of clock (also costs a life), ! = a drop of 1500+ from the rung before.\n");
}

/// How much of the ladder the fountains are carrying.
///
/// `ladder_curve` fights classless, because a profile never visits a fountain.
/// That is half the run's scaling left out, so the flat ceiling it reports is
/// a floor rather than a verdict. This gives the same board the classes it
/// would actually qualify for and walks the ladder again - the gap between the
/// two lines is what the fountains are worth.
#[test]
#[ignore]
fn what_classes_are_worth() {
    use gearmaster_engine::combat::Difficulty;
    use gearmaster_engine::run::Mode;

    let layout: Layout =
        SlotKind::ALL.iter().map(|&s| (s, pack_dense(s, |d| d.rating))).collect();

    // What this board is actually offered at a fountain, best first.
    let outlook = {
        let run = wear_layout(&layout);
        run.class_outlook()
    };
    let offered: Vec<&'static str> =
        outlook.iter().filter(|m| m.eligible).map(|m| m.class.name).collect();
    println!("\n=== what the fountains are worth ===");
    println!("the dense board qualifies for: {}", if offered.is_empty() {
        "nothing".to_string()
    } else {
        offered.join(", ")
    });

    // Each class on its own, so a dead one is visible as a dead one. Taking
    // the first three the fingerprint offers hid this completely: they were
    // all cast powers on a board that casts nothing, and the margin came out
    // identical to the unit at all fifty rungs.
    let at = [14usize, 19, 24, 29, 34];
    print!("\n{:<16}", "class");
    for r in at {
        print!("{:>10}", format!("r{}", r + 1));
    }
    println!("   power");

    let mut base = Vec::new();
    for &r in &at {
        let mut run = wear_layout(&layout);
        run.difficulty = Difficulty::Medium;
        run.mode = Mode::Grinder;
        run.rung = r;
        let log = run.fight_next();
        let (ph, eh) = final_health(log);
        base.push(ph - eh);
    }
    print!("{:<16}", "(none)");
    for m in &base {
        print!("{:>10}", m);
    }
    println!();

    for m in outlook.iter().filter(|m| m.eligible) {
        print!("{:<16}", m.class.name);
        for (i, &r) in at.iter().enumerate() {
            let mut run = wear_layout(&layout);
            run.difficulty = Difficulty::Medium;
            run.mode = Mode::Grinder;
            run.classes.push(m.class);
            run.rung = r;
            let log = run.fight_next();
            let (ph, eh) = final_health(log);
            let d = (ph - eh) - base[i];
            print!("{:>10}", if d == 0 { "-".to_string() } else { format!("{:+}", d) });
        }
        println!("   {:?}", m.class.power);
    }
    println!("\n'-' means the class changed nothing at all.\n");
}

/// Fit one named component into ten creatures that do not have it.
///
/// Hand-placing these is how you ship a monster whose new gear silently does
/// nothing: it lands somewhere legal, joins the nearest core, and quietly
/// turns a working item into an over-full one that assembles into neither
/// recipe. Everything the creature already owns is locked first, so the
/// addition fits around it or is not made at all.
#[test]
#[ignore]
fn author_the_pool_drains() {
    use gearmaster_engine::combat::{Rank, LADDER};

    // The drain piece, and the smallest legal item that can carry it. A
    // creature with no room for a four-piece helmet may still have room for a
    // three-piece one.
    let carriers: &[(&str, SlotKind, &[&str])] = &[
        ("Tithe Collector", SlotKind::Helmet, &["Bone Frame", "Tin Plating", "Tithe Collector"]),
        ("Wrathbreaker", SlotKind::Chest, &["Grove Base", "Wrathbreaker"]),
        ("Witherroot", SlotKind::Greaves, &["Rootwoven Material", "Witherroot"]),
        ("Manaflay", SlotKind::Weapon, &["Oak Handle", "Iron Blade", "Manaflay"]),
    ];

    // Ordinary creatures only - no boss, no mini-boss - and from rung 18 up,
    // where a player has pools worth taking. Every third one, so the answer is
    // spread across the back half rather than bunched.
    let victims: Vec<&'static str> = LADDER
        .iter()
        .enumerate()
        .filter(|(i, m)| *i >= 17 && m.rank == Rank::Ordinary)
        .map(|(_, m)| m.name)
        .step_by(2)
        .take(10)
        .collect();
    assert_eq!(victims.len(), 10, "not enough ordinary creatures deep enough to carry these");

    println!("\n=== pool drains, one per creature ===\n");
    for (n, name) in victims.iter().enumerate() {
        let m = LADDER.iter().find(|m| m.name == *name).expect("on the ladder");
        // Rotate through the four so no one pool is the only one answered.
        let (piece, slot, recipe) = carriers[n % carriers.len()];

        // No locking. Every one of these creatures carries `items: &[]`,
        // which tells the loader to place the whole board in one go and let
        // the core-anchoring work it out - so a placement that only holds
        // because the neighbours were locked is a placement the loader will
        // not reproduce. `seat` searches under the same rules, and only
        // returns a spot that leaves *every* item in the slot assembled.
        let (mut reg, mut loadout) = m.loadout();
        assert!(
            m.items.is_empty(),
            "{name} pins its item boundaries; appending gear needs `items` extended too"
        );
        // `loadout_at` locks what it built before handing it back. The loader
        // does that *after* the whole board is down, so a placement found
        // against a locked board is one the loader will not reproduce - it
        // will re-derive from nearest-core with the new pieces already there
        // and hand a crest to an item that has one.
        loadout.locks.clear();
        let before = loadout.report(&reg, slot).items.iter().filter(|i| i.assembled).count();

        let ids: Vec<PieceId> = recipe
            .iter()
            .map(|n| reg.alloc(CATALOG.iter().position(|d| d.name == *n).expect(n)))
            .collect();
        let Some(spots) = seat(&mut reg, &mut loadout, slot, &ids, 0) else {
            println!("// {name}: no room for {piece} in {slot:?}");
            continue;
        };
        let after = loadout.report(&reg, slot).items.iter().filter(|i| i.assembled).count();
        assert_eq!(after, before + 1, "{name}: seating {piece} did not add an item");

        println!("// {name} - {piece}");
        for (i, &(x, y, rot)) in spots.iter().enumerate() {
            println!(
                "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
                recipe[i], slot, x, y, rot
            );
        }
        println!();
    }
}
