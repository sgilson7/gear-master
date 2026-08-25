//! Packing mode: the game, dressing somebody else.
//!
//! `pack_francis` searches for a creature's board against a difficulty curve.
//! It works, and it takes about five minutes per creature per power band -
//! half an hour a board - and what it finds satisfies a number rather than
//! being a board somebody meant. A person looking at the grid is faster and
//! packs more thematically, so this is the grid, and the person is the search.
//!
//! The first attempt at this was a separate window with its own grids, its own
//! click-to-place and no tooltips, and it was out of parity with the game
//! before it was finished. This is the correction: **there is no second
//! interface.** `GEARMASTER_PACK=1` loads a creature's board into the run's own
//! loadout, so every part of the screen the player already knows - dragging,
//! locking, item outlines, hover tooltips, the recipe hints - is the same code
//! doing the same thing. What changes is three things: whose board it is, a
//! shop that stocks the whole catalogue for nothing, and a save.
//!
//! **It edits `crates/engine/src/combat.rs` in place.** Saving rewrites the
//! selected `MonsterSpec`'s `gear:` and `items:` and leaves the rest of the
//! literal alone - health, attacks, bounty, sprite, rank, drops. The tool
//! authors a board, not a creature.
//!
//! Loading needs no parsing: `LADDER` and `ALTERNATES` are compiled in, so it
//! always opens on whatever the source says today.

use gearmaster_engine::combat::{
    stepped_component, Difficulty, GearPlacement, MonsterSpec, ALTERNATES, LADDER,
};
use gearmaster_engine::loadout::lock_assembled_in;
use gearmaster_engine::piece::{
    is_boss_only, is_event_only, is_quest_reward, is_town_stock, PieceId, SlotKind, CATALOG,
};
use gearmaster_engine::run::Run;

/// Every creature in the game, ladder first, in the order they are written.
pub fn everyone() -> Vec<&'static MonsterSpec> {
    LADDER.iter().chain(ALTERNATES.iter()).collect()
}

/// Where packing keeps what the game has nowhere to put.
pub struct Pack {
    /// Which creature is being dressed, indexing `everyone()`.
    pub who: usize,
    /// What the last thing that happened was.
    pub status: String,
    /// The catalogue search, and which page of it the shelves are showing.
    pub search: String,
    pub page: usize,
    /// Whether the creature picker is up.
    pub picking: bool,
    /// Which setting the grids are showing.
    ///
    /// Medium is the board. The other three are what `stepped_component` makes
    /// of it, and are shown rather than edited - there is nothing to edit in a
    /// board nobody wrote, and stepping is not invertible, so an edit made on
    /// Hard could not be written back to the thing Hard was derived from.
    pub setting: Difficulty,
    /// The authored board, held while a stepped one is on the grids so it can
    /// be put back exactly.
    pub authored: Option<(Vec<GearPlacement>, Vec<usize>)>,
}

impl Default for Pack {
    fn default() -> Self {
        Pack {
            who: 0,
            status: String::new(),
            search: String::new(),
            page: 0,
            picking: false,
            setting: Difficulty::Medium,
            authored: None,
        }
    }
}

impl Pack {
    pub fn spec(&self) -> &'static MonsterSpec {
        everyone()[self.who.min(everyone().len() - 1)]
    }

    /// Is what is on the grids the board, or a step of it?
    pub fn editing(&self) -> bool {
        self.setting == Difficulty::Medium
    }
}

/// Show `run`'s board at `want`, keeping the authored one safe.
///
/// Switching away from Medium stashes what is on the grids and puts the stepped
/// board there instead; switching back puts the original one down again exactly
/// as it was. Anything done to a stepped board is discarded on the way back,
/// which is the honest outcome: stepping is not invertible, so there is nowhere
/// for such an edit to go.
pub fn show_at(run: &mut Run, pk: &mut Pack, want: Difficulty) {
    if want == pk.setting {
        return;
    }
    if pk.authored.is_none() {
        pk.authored = Some(emit_gear(run));
    }
    let (gear, chunks) = pk.authored.clone().expect("just stashed");
    let show = if want == Difficulty::Medium { gear.clone() } else { stepped(&gear, want) };
    load_gear_into(run, &show, &chunks);
    pk.setting = want;
    if want == Difficulty::Medium {
        pk.authored = None;
    }
}

/// Put a creature's board on the run's own grids.
///
/// Chunk by chunk, locking each one before the next goes down - which is what
/// `MonsterSpec::loadout_at` does and is not cosmetic. An unlocked board
/// negotiates with itself: two items packed flush merge into one over-full
/// thing that assembles into nothing. Loading without it showed forty-four of
/// the fifty-four holding different items from the ones they are written as
/// holding.
pub fn load_into(run: &mut Run, spec: &MonsterSpec) -> usize {
    load_gear_into(run, spec.gear, spec.items)
}

/// The same, from a gear list that is not a `MonsterSpec`'s own - which is what
/// a stepped board is, since stepping rewrites every name.
pub fn load_gear_into(run: &mut Run, gear: &[GearPlacement], items: &[usize]) -> usize {
    for kind in SlotKind::ALL {
        run.loadout.slot_mut(kind).clear();
    }
    run.loadout.locks.clear();
    run.owned.clear();
    let mut dropped = 0;

    let mut chunks: Vec<usize> = items.to_vec();
    if chunks.is_empty() {
        chunks = vec![gear.len()];
    }
    let mut at = 0usize;
    for take in chunks {
        let end = (at + take).min(gear.len());
        let mut touched: Vec<SlotKind> = Vec::new();
        for &(name, slot, x, y, rot) in &gear[at..end] {
            let Some(def) = CATALOG.iter().position(|d| d.name == name) else {
                dropped += 1;
                continue;
            };
            let id = run.registry.alloc(def);
            run.registry.set_rotation(id, rot);
            run.owned.push(id);
            if run.loadout.can_place(&run.registry, id, slot, x, y).is_ok() {
                run.loadout.slot_mut(slot).place(&run.registry, id, x, y);
                if !touched.contains(&slot) {
                    touched.push(slot);
                }
            } else {
                dropped += 1;
            }
        }
        for kind in touched {
            lock_assembled_in(&mut run.loadout, &run.registry, kind);
        }
        at = end;
    }
    dropped
}

/// The board as the fight would read it: placements in item order, and the
/// partition that keeps them apart.
///
/// `items` is a run-length partition of `gear` and the fight relies on it:
/// `MonsterSpec::loadout_at` seats each chunk and locks it before the next one
/// lands, which is the only reason a densely packed board holds its shape. So
/// every item's pieces come out contiguously and the chunks sum to the length
/// of the list.
///
/// Written order is kept wherever it can be. Ids are handed out in the order
/// the gear list was read, so the lowest id in a group is where that group
/// stood in the file; sorting on it means opening a creature and saving it
/// produces no diff at all, and anything newly bought has a high id and lands
/// at the end. Without it every one of the fifty-four came out reordered the
/// first time it was touched, and a fifty-line diff for a change nobody made is
/// a diff nobody reads.
pub fn emit_gear(run: &Run) -> (Vec<GearPlacement>, Vec<usize>) {
    let mut groups: Vec<(u32, SlotKind, Vec<PieceId>)> = Vec::new();
    for slot in SlotKind::ALL {
        let report = run.loadout.report(&run.registry, slot);
        let mut seated: Vec<PieceId> = Vec::new();
        for item in report.items.iter().filter(|i| i.assembled) {
            let first = item.pieces.iter().map(|p| p.0).min().unwrap_or(u32::MAX);
            // And within the item too: `report` hands its pieces back in board
            // order, which is not the order they were written in.
            let mut pieces = item.pieces.clone();
            pieces.sort_by_key(|p| p.0);
            groups.push((first, slot, pieces));
            seated.extend(item.pieces.iter().copied());
        }
        // Loose gear last, one chunk each. A chunk of one locks a single piece,
        // which is harmless; leaving it out is a sum that does not match.
        for id in run.loadout.slot(slot).pieces() {
            if seated.contains(&id) || run.registry.def(id).kind.is_enchantment() {
                continue;
            }
            groups.push((id.0, slot, vec![id]));
        }
    }
    groups.sort_by_key(|(first, ..)| *first);

    let mut gear = Vec::new();
    let mut chunks = Vec::new();
    for (_, slot, pieces) in groups {
        chunks.push(pieces.len());
        for p in pieces {
            let (x, y) = run.loadout.slot(slot).anchor_of(p).unwrap_or((0, 0));
            gear.push((run.registry.def(p).name, slot, x, y, run.registry.rotation(p)));
        }
    }
    (gear, chunks)
}

/// The same board, stepped for a setting - which is what the creature actually
/// wears there. Medium is the identity.
pub fn stepped(gear: &[GearPlacement], d: Difficulty) -> Vec<GearPlacement> {
    let step = d.gear_step();
    gear.iter()
        .map(|&(n, s, x, y, r)| (stepped_component(n, step), s, x, y, r))
        .collect()
}

/// Can this board land anything at all?
///
/// "Weapons swing; everything else just does its job" (`combat.rs`), so a
/// creature holding no weapon has no offence except what its triggers do. Four
/// of the six themes hold no weapon, and seven creatures once stood on the road
/// at Medium doing nothing whatsoever because of it - which nothing noticed,
/// because the test that asks reads `simulate`, and `simulate` is Easy.
///
/// Read off the gear rather than simulated, so it can run every frame at every
/// setting. Innate attacks count; a finished weapon item counts; and any
/// trigger that damages, eats maximum health or burns counts wherever it sits -
/// as does the `mind` stat, which is not an action and is how The Rust
/// Parliament lands fifteen blows on Easy from a board whose every trigger is a
/// drain.
pub fn can_hurt(spec: &MonsterSpec, gear: &[GearPlacement], chunks: &[usize]) -> bool {
    use gearmaster_engine::curse::CurseKind;
    use gearmaster_engine::piece::{walk_actions, Action};
    if !spec.attacks.is_empty() {
        return true;
    }
    // Assembled-ness is what decides, so the board has to be laid out again.
    let mut probe = Run::new();
    let dressed = MonsterSpec {
        gear: Box::leak(gear.to_vec().into_boxed_slice()),
        items: Box::leak(chunks.to_vec().into_boxed_slice()),
        ..*spec
    };
    load_into(&mut probe, &dressed);
    for slot in SlotKind::ALL {
        let report = probe.loadout.report(&probe.registry, slot);
        for item in report.items.iter().filter(|i| i.assembled) {
            if slot == SlotKind::Weapon {
                return true;
            }
            for &p in &item.pieces {
                let def = probe.registry.def(p);
                if def.base.mind != 0 {
                    return true;
                }
                let mut lands = false;
                for t in def.triggers {
                    walk_actions(t, &mut |a| {
                        if matches!(
                            a,
                            Action::Damage { .. }
                                | Action::MindDamage { .. }
                                | Action::Curse { kind: CurseKind::Searing, .. }
                        ) {
                            lands = true;
                        }
                    });
                }
                if lands {
                    return true;
                }
            }
        }
    }
    false
}

/// Everything wrong with this board, worst first, in the words the suite uses.
///
/// The same checks the tests make, run while somebody is still looking at the
/// grid - which is most of the reason to have a tool rather than a search. The
/// search found every one of these out one failing test at a time.
pub fn complaints(run: &Run, spec: &MonsterSpec) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let (gear, chunks) = emit_gear(run);

    // Whether it lands anything, at every setting, not just the one on screen.
    for &d in Difficulty::ALL {
        let walked = stepped(&gear, d);
        if !can_hurt(spec, &walked, &chunks) {
            out.push((format!("lands nothing at all on {}", d.name()), true));
        }
    }

    let mut worn: Vec<SlotKind> = Vec::new();
    for slot in SlotKind::ALL {
        let report = run.loadout.report(&run.registry, slot);
        let items = report.items.iter().filter(|i| i.assembled).count();
        if items > 0 {
            worn.push(slot);
        }
        for id in run.loadout.slot(slot).pieces() {
            let def = run.registry.def(id);
            if def.kind.is_enchantment() {
                out.push((format!("{} is enchanted, and creatures are not", def.name), true));
            } else if is_event_only(def.name) {
                out.push((format!("{} is what a door hands over", def.name), true));
            } else if is_town_stock(def) {
                out.push((format!("{} is sold in a town", def.name), true));
            } else if is_quest_reward(def.name) {
                out.push((format!("{} is the far side of a quest", def.name), false));
            } else if is_boss_only(def.name) {
                // A trophy belongs to exactly one creature, by wearing it *or*
                // dropping it - `boss_gear_belongs_to_exactly_one_monster`
                // counts both. Francis wears The Money Jacket and drops
                // nothing, so a check that only read `drops` called the one
                // strange thing he owns somebody else's.
                let taken: Vec<&str> = everyone()
                    .into_iter()
                    .filter(|m| m.name != spec.name)
                    .filter(|m| {
                        m.gear.iter().any(|(n, ..)| *n == def.name)
                            || m.drops.contains(&def.name)
                    })
                    .map(|m| m.name)
                    .collect();
                if let Some(owner) = taken.first() {
                    out.push((format!("{} belongs to {owner}", def.name), true));
                }
            }
        }
        // A creature swings once a cooldown. Two weapon items is two swings.
        if slot == SlotKind::Weapon && items > 1 {
            out.push((format!("{items} weapon items is {items} swings a cooldown"), true));
        }
        let owed = spec.rank.min_items_in(slot);
        if items > 0 && items < owed {
            out.push((
                format!("{} holds {items} item(s); a {:?} owes {owed}", slot.name(), spec.rank),
                false,
            ));
        }
    }
    if spec.rank.is_named() && worn.len() < spec.rank.min_slots() {
        out.push((
            format!("wears {} slot(s); a {:?} owes {}", worn.len(), spec.rank, spec.rank.min_slots()),
            false,
        ));
    }
    out
}

// ------------------------------------------------------------------- saving

const COMBAT_RS: &str = "crates/engine/src/combat.rs";

/// Write the board back where the creature is written.
pub fn save(run: &Run, spec: &MonsterSpec) -> Result<String, String> {
    let (gear, chunks) = emit_gear(run);
    let lines: Vec<String> = gear
        .iter()
        .map(|&(name, slot, x, y, rot)| {
            format!("            (\"{name}\", SlotKind::{slot:?}, {x}, {y}, {rot}),")
        })
        .collect();
    let src = std::fs::read_to_string(COMBAT_RS).map_err(|e| format!("{COMBAT_RS}: {e}"))?;
    let out = splice(&src, spec.name, &lines, &chunks)?;
    std::fs::write(COMBAT_RS, &out).map_err(|e| format!("{COMBAT_RS}: {e}"))?;
    Ok(format!(
        "saved {}: {} pieces in {} item(s) - rebuild to see it in the game",
        spec.name,
        lines.len(),
        chunks.len()
    ))
}

/// Rewrite one `MonsterSpec`'s `gear:` and `items:` where they sit.
///
/// Anchored on the `name:` line and on the indent of the literal it belongs to,
/// which is how the block's own terminator is found - `gear:` is a nested array
/// and the first `],` inside it belongs to a `cells:` entry, not to the block.
///
/// `items:` is rewritten before `gear:` because it comes later in the file, so
/// splicing it first leaves the earlier offsets alone.
pub fn splice(
    src: &str,
    name: &str,
    lines: &[String],
    chunks: &[usize],
) -> Result<String, String> {
    let needle = format!("name: \"{name}\",");
    let at = src.find(&needle).ok_or_else(|| format!("no creature called {name}"))?;
    let line_start = src[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let indent: String = src[line_start..at].to_string();

    let g = src[at..].find("gear: &[").ok_or("that creature has no gear list")? + at;
    // An empty list is written on one line - `gear: &[],` - and its terminator
    // is right there rather than at the literal's indent. Searching for the
    // indented one from an empty list walks straight past the creature and
    // finds the *next* one's, which is how a creature with teeth and no gear
    // came out of this looking like a file in the wrong order.
    let empty = src[g..].starts_with("gear: &[],");
    let end_marker = if empty { "],".to_string() } else { format!("\n{indent}],") };
    let ge = src[g..].find(&end_marker).ok_or("the gear list does not end")? + g;

    let i = src[at..].find("items: &[").ok_or("that creature has no items list")? + at;
    let ie = src[i..].find(']').ok_or("the items list does not end")? + i;

    if !(g < ge && ge < i && i < ie) {
        return Err("gear and items are not in the order this expects".into());
    }

    let gear_block = if lines.is_empty() {
        "gear: &[],".to_string()
    } else {
        format!("gear: &[\n{}\n{indent}],", lines.join("\n").trim_end_matches('\n'))
    };
    let items_block = format!("items: &{chunks:?}");

    let mut out = String::with_capacity(src.len() + 512);
    out.push_str(&src[..i]);
    out.push_str(&items_block);
    out.push_str(&src[ie + 1..]);
    let src = out;

    let mut out = String::with_capacity(src.len() + 512);
    out.push_str(&src[..g]);
    out.push_str(&gear_block);
    out.push_str(&src[ge + end_marker.len()..]);
    Ok(out)
}

// ------------------------------------------------------------- the shelves

/// Everything the shop should be offering, given the search.
///
/// No gold, no availability, no restock: a packer's shop is the catalogue with
/// a filter on it. What it does keep out is what a creature may not wear, which
/// is the one part of the real shop's reasoning that still applies.
pub fn shelf(search: &str) -> Vec<usize> {
    let needle = search.to_lowercase();
    (0..CATALOG.len())
        .filter(|&i| {
            let d = &CATALOG[i];
            if d.kind.is_enchantment() || is_event_only(d.name) || is_town_stock(d) {
                return false;
            }
            if needle.is_empty() {
                return true;
            }
            d.name.to_lowercase().contains(&needle)
                || d.kind.name().contains(&needle)
                || d.slot.name().to_lowercase().contains(&needle)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> String {
        std::fs::read_to_string(COMBAT_RS)
            .or_else(|_| std::fs::read_to_string(format!("../../{COMBAT_RS}")))
            .expect("the engine source, from the workspace root or this crate")
    }

    /// The whole of packing, minus the window: every creature in the game, read
    /// off its `MonsterSpec`, laid on a board, read back out, and spliced into
    /// the source - and the source has to come out saying the same thing.
    ///
    /// This is the test the tool exists behind. A board editor whose save is
    /// not the identity on a board nobody touched is an editor that quietly
    /// rewrites fifty-four creatures the first time it is opened.
    #[test]
    fn opening_a_creature_and_saving_it_changes_nothing() {
        let src = source();
        let mut moved = Vec::new();
        for spec in everyone() {
            let mut run = Run::new();
            let dropped = load_into(&mut run, spec);
            assert_eq!(dropped, 0, "{}: {dropped} placement(s) would not sit", spec.name);
            let (gear, chunks) = emit_gear(&run);
            assert_eq!(
                gear.len(),
                spec.gear.len(),
                "{}: {} pieces went in and {} came out",
                spec.name,
                spec.gear.len(),
                gear.len()
            );
            assert_eq!(
                chunks.iter().sum::<usize>(),
                gear.len(),
                "{}: the item chunks do not partition the gear list",
                spec.name
            );
            let lines: Vec<String> = gear
                .iter()
                .map(|&(n, s, x, y, r)| format!("            (\"{n}\", SlotKind::{s:?}, {x}, {y}, {r}),"))
                .collect();
            if splice(&src, spec.name, &lines, &chunks).expect("splices") != src {
                moved.push(spec.name);
            }
        }
        assert!(
            moved.len() <= REORDERED_ON_FIRST_SAVE,
            "{} creatures would be rewritten by an untouched save, budget {}: {:?}",
            moved.len(),
            REORDERED_ON_FIRST_SAVE,
            moved
        );
    }

    /// How many creature boards packing would rewrite if you opened them and
    /// saved without touching anything.
    ///
    /// It was fifty-two of fifty-four, for three reasons in a row, each of them
    /// a thing the tool was getting wrong about creature boards: items came out
    /// in slot order rather than written order; the board was loaded without
    /// locking each item as it landed, so flush items merged; and the pieces
    /// inside an item came back in board order. Six left, and those six have an
    /// `items:` partition that does not describe the board it is attached to -
    /// saving one of those is the fix rather than the noise.
    ///
    /// Lower this when one of the six is fixed. It should never rise.
    const REORDERED_ON_FIRST_SAVE: usize = 6;

    /// The check that would have saved a day, held to the answer the engine
    /// suite gets by actually fighting.
    #[test]
    fn nothing_on_the_ladder_is_toothless_at_any_setting() {
        let mut bad = Vec::new();
        for spec in everyone() {
            let mut run = Run::new();
            load_into(&mut run, spec);
            let (gear, chunks) = emit_gear(&run);
            for &d in Difficulty::ALL {
                if !can_hurt(spec, &stepped(&gear, d), &chunks) {
                    bad.push(format!("{} on {}", spec.name, d.name()));
                }
            }
        }
        assert!(bad.is_empty(), "boards that land nothing: {bad:?}");
    }

    /// And that the check is not simply always true.
    #[test]
    fn a_board_with_nothing_on_it_is_reported_as_toothless() {
        let spec = LADDER.iter().find(|m| m.name == "Bog Toad").expect("a toad");
        assert!(!can_hurt(spec, &[], &[]), "an empty board was called dangerous");
        let mut run = Run::new();
        load_into(&mut run, spec);
        let (gear, chunks) = emit_gear(&run);
        assert!(can_hurt(spec, &gear, &chunks), "the toad's own board was called harmless");
    }

    #[test]
    fn a_splice_leaves_everything_but_the_board_alone() {
        let src = source();
        let out = splice(
            &src,
            "Cave Rat",
            &["            (\"Oak Handle\", SlotKind::Weapon, 0, 0, 0),".to_string()],
            &[1],
        )
        .expect("splices");
        assert_eq!(
            src.matches("MonsterSpec {").count(),
            out.matches("MonsterSpec {").count(),
            "the splice added or removed a creature"
        );
        assert!(out.contains("name: \"Cave Rat\","), "it lost the name it was aiming at");
        assert!(out.contains("(\"Oak Handle\", SlotKind::Weapon, 0, 0, 0),"), "the gear did not land");
        assert!(out.contains("items: &[1]"), "the chunks did not land");
    }

    #[test]
    fn a_creature_that_is_not_there_is_refused_rather_than_guessed_at() {
        let err = splice("nothing here", "Nobody", &[], &[]).unwrap_err();
        assert!(err.contains("Nobody"), "{err}");
    }

    /// A packer's shelf is the catalogue with a filter on it - but it still
    /// keeps out what a creature may not wear.
    #[test]
    fn the_shelf_offers_everything_a_creature_may_wear_and_nothing_else() {
        let all = shelf("");
        assert!(all.len() > 400, "only {} components on the shelf", all.len());
        for &i in &all {
            let d = &CATALOG[i];
            assert!(!d.kind.is_enchantment(), "{} is an enchantment", d.name);
            assert!(!is_event_only(d.name), "{} is event gear", d.name);
            assert!(!is_town_stock(d), "{} is town stock", d.name);
        }
        let oaken = shelf("oak");
        assert!(!oaken.is_empty());
        for &i in &oaken {
            assert!(CATALOG[i].name.to_lowercase().contains("oak"));
        }
        // Searching by slot and by kind, because a name is not the only thing
        // somebody knows about the piece they are after.
        assert!(!shelf("greaves").is_empty(), "searching by slot found nothing");
        assert!(!shelf("plating").is_empty(), "searching by kind found nothing");
    }
}
