//! A search that lays a set of components out in a slot so that every item in
//! it assembles. Used to author the bosses: hand-placing polyominoes and
//! hoping the core-anchoring split falls the right way is a good way to ship a
//! boss whose gear silently does nothing.
//!
//! `cargo test -p gearmaster-engine --test packing -- --ignored --nocapture`
//! prints gear tuples ready to paste into `LADDER`.

use gearmaster_engine::loadout::Loadout;
use gearmaster_engine::piece::{PieceId, PieceKind, PieceRegistry, SlotKind, CATALOG};
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
        assert_eq!(CATALOG[d].slot, slot, "{} is not a {} piece", n, slot.name());
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
    const POOL_CAP: usize = 6;
    let mut per_kind: Vec<Vec<Vec<usize>>> = Vec::new();
    for &(kind, min, max) in recipe {
        let mut pool: Vec<usize> = (0..CATALOG.len())
            // `fits`, not `slot ==`: materials are shared between gloves and
            // greaves and plating between helmets and greaves, so keying on
            // the home slot hid 22 of the 46 pieces a greave can take.
            .filter(|&i| CATALOG[i].fits(slot) && CATALOG[i].kind == kind)
            // A disconnected shape can never be part of an assembled item:
            // its islands flood-fill into groups of their own.
            .filter(|&i| connected(CATALOG[i].cells))
            .collect();
        pool.sort_by_key(|&i| std::cmp::Reverse(piece_rating(&CATALOG[i])));
        pool.truncate(POOL_CAP);
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
            Axis::Brutality => (s.physical_damage + s.damage) as f32,
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
                    (s.physical_damage + s.damage + s.physical_resist + s.physical_pierce) as f32
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

        let mut best: Option<(f32, Vec<(&'static str, u8, u8, u8)>)> = None;
        for (score, names) in scored.into_iter().take(150) {
            if let Some(p) = pack(slot, &names) {
                best = Some((score, p));
                break;
            }
        }
        if let Some((_, placed)) = best {
            if !wear(&mut run, slot, &placed) {
                println!("  (could not fit {} for {})", slot.name(), class.name);
            }
        }
    }
    run
}

#[test]
#[ignore]
fn which_classes_are_reachable() {
    println!("\n=== can a real build reach each class? ===");
    let mut dead = Vec::new();
    for class in CLASSES {
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

/// Fill a grid until nothing else will go in.
///
/// The other profiles hand the packer a fixed shopping list and take whatever
/// layout it finds, which caps a slot at one or two items however much room is
/// left over. A player does not build that way - they keep dropping things in
/// until the grid is full. This adds items one at a time, keeping whatever
/// still lets every item in the slot assemble, and stops when nothing fits.
fn pack_dense(slot: SlotKind, rank: impl Fn(&PieceDefRef) -> i32) -> Vec<(&'static str, u8, u8, u8)> {
    let mut ranked: Vec<(i32, Vec<&'static str>)> = candidates(slot)
        .into_iter()
        .filter(|(_, names)| {
            // A run owns one of each, so a layout wanting two of something
            // cannot be worn.
            let mut seen: Vec<&str> = Vec::new();
            names.iter().all(|n| if seen.contains(n) { false } else { seen.push(n); true })
        })
        .map(|(r, names)| {
            let cells: i32 = names
                .iter()
                .map(|n| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len() as i32)
                .sum();
            let cooldown_ms = candidate_cooldown(slot, &names);
            (rank(&PieceDefRef { rating: r, cells, cooldown_ms }), names)
        })
        .collect();
    ranked.sort_by_key(|(r, _)| std::cmp::Reverse(*r));
    ranked.truncate(90);

    let mut reg = PieceRegistry::new();
    let mut loadout = Loadout::new();
    let mut placed: Vec<(&'static str, u8, u8, u8)> = Vec::new();
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
            // `seat` only returns a placement that leaves every item in the
            // slot assembled, so anything it hands back is keepable.
            if let Some(spots) = seat(&mut reg, &mut loadout, slot, &ids, 0) {
                for (i, &(x, y, rot)) in spots.iter().enumerate() {
                    placed.push((names[i], x, y, rot));
                }
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

fn choose(profile: Profile, slot: SlotKind, seed: u64) -> Option<Vec<(&'static str, u8, u8, u8)>> {
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
                return Some(
                    p.into_iter()
                        .enumerate()
                        .map(|(i, (n, x, y, _))| (n, x, y, ((seed as u8) + i as u8) % 4))
                        .collect(),
                );
            }
            return Some(p);
        }
    }
    None
}

/// Rungs to test at, rather than walking all 33. A full walk meant thousands
/// of simulations; five spot checks answer the same question - how deep does
/// this kind of player get - in a fraction of the time.
const BREAKPOINTS: [usize; 5] = [0, 8, 16, 24, 32];

/// One profile's board, packed once.
type Layout = Vec<(SlotKind, Vec<(&'static str, u8, u8, u8)>)>;

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
                    SlotKind::ALL
                        .iter()
                        .filter_map(|&slot| {
                            choose(profile, slot, seed + slot.index() as u64).map(|p| (slot, p))
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// Wear a prepared board and see which breakpoints it can beat.
fn play_layout(layout: &Layout, difficulty: Difficulty) -> Vec<bool> {
    use gearmaster_engine::combat::Outcome;
    use gearmaster_engine::run::{Mode, Run};
    let mut run = Run::with_all_pieces();
    run.difficulty = difficulty;
    run.mode = Mode::Grinder;
    for (slot, placed) in layout {
        for (name, x, y, rot) in placed {
            if let Some(id) = run
                .owned
                .iter()
                .copied()
                .find(|&i| run.registry.def(i).name == *name && !run.is_equipped(i))
            {
                run.registry.set_rotation(id, *rot);
                // A turned piece may no longer fit. That is the non-assembler
                // profile working, not a failure.
                let _ = run.equip(id, *slot, *x, *y);
            }
        }
    }
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
    use gearmaster_engine::run::Run;
    let mut run = Run::with_all_pieces();
    for (slot, placed) in layout {
        for (name, x, y, rot) in placed {
            if let Some(id) = run
                .owned
                .iter()
                .copied()
                .find(|&i| run.registry.def(i).name == *name && !run.is_equipped(i))
            {
                run.registry.set_rotation(id, *rot);
                let _ = run.equip(id, *slot, *x, *y);
            }
        }
    }
    let items = run.combat_items();
    let stats = run.player_stats();
    let dps: i64 = items.iter().map(|i| i.dps_milli(stats.strength, stats.power)).sum();
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
        let cells = |p: &[(&'static str, u8, u8, u8)]| -> usize {
            p.iter()
                .map(|(n, ..)| CATALOG.iter().find(|c| c.name == *n).unwrap().cells.len())
                .sum()
        };
        let names = |p: &[(&'static str, u8, u8, u8)]| -> Vec<&str> {
            let mut v: Vec<&str> = p.iter().map(|(n, ..)| *n).collect();
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
