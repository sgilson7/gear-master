//! A search that lays a set of components out in a slot so that every item in
//! it assembles. Used to author the bosses: hand-placing polyominoes and
//! hoping the core-anchoring split falls the right way is a good way to ship a
//! boss whose gear silently does nothing.
//!
//! `cargo test -p gearmaster-engine --test packing -- --ignored --nocapture`
//! prints gear tuples ready to paste into `LADDER`.

use gearmaster_engine::loadout::Loadout;
use gearmaster_engine::piece::{PieceId, PieceRegistry, SlotKind, CATALOG};
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
    use gearmaster_engine::rating::piece_rating;

    // Choose `n` from `pool` with repetition, as sorted index lists.
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

    let mut per_kind: Vec<Vec<Vec<usize>>> = Vec::new();
    for &(kind, min, max) in gearmaster_engine::piece::recipe(slot) {
        let pool: Vec<usize> = (0..CATALOG.len())
            .filter(|&i| CATALOG[i].slot == slot && CATALOG[i].kind == kind)
            // A disconnected shape can never be part of an assembled item:
            // its islands flood-fill into groups of their own.
            .filter(|&i| connected(CATALOG[i].cells))
            .collect();
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
