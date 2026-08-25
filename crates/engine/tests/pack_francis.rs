//! A packing search, run by hand to author a creature's boards.
//!
//! Ignored by default: it is a generator, not a check. `cargo test -p
//! gearmaster-engine --test pack_francis -- --ignored --nocapture` prints a
//! `gear` and `items` list to paste into `combat.rs`.
//!
//! Why it exists: the two finished human boards in `share` pack ninety-seven
//! and ninety-eight percent of their cells, and Francis - the last thing on
//! the ladder - was at thirty-six, with one item per slot. He was not losing
//! because he was weak, he was losing because four fifths of his boards were
//! empty. Hand-authoring seventy-odd placements across five grids is not
//! something to do in a text editor.
//!
//! The rule the search has to respect is `MonsterSpec::unassembled`: every
//! chunk a creature's gear is cut into must come together into a real item. A
//! player may seat loose pieces for their flat stats - the friend's board does
//! it twelve times - but a creature may not.

use gearmaster_engine::loadout::{lock_assembled_in, Loadout};
use gearmaster_engine::piece::{
    is_boss_only, recipes, PieceDef, PieceKind, PieceRegistry, SlotKind, CATALOG,
};
use gearmaster_engine::rating::piece_rating;
use gearmaster_engine::rng::Rng;
use gearmaster_engine::slot::{SLOT_H, SLOT_W};

mod common;

/// Which creature is being packed. Francis by default, because he is the one
/// this search was written for and the one whose board is hardest to author.
fn who() -> String {
    std::env::var("PACK_MONSTER").unwrap_or_else(|_| "Francis".into())
}

/// The one boss trophy the creature being packed is allowed to wear.
///
/// A trophy belongs to exactly one creature - it is the thing that creature
/// leaves behind - so Francis may wear his coat and nobody else may. A monster
/// with no trophy of its own passes an empty string, which matches nothing.
fn mine() -> String {
    std::env::var("PACK_TROPHY").unwrap_or_else(|_| {
        if who() == "Francis" { "The Money Jacket".into() } else { String::new() }
    })
}

/// How far down the rating order to start drawing.
///
/// Packing a board to ninety-five percent with the *best* piece of every kind
/// does not make a hard fight, it makes an impossible one: the first attempt
/// killed both finished human boards in under three seconds at every setting,
/// and dropping Francis's own health and strength by three quarters changed
/// nothing, because none of the damage was coming from him. Density and power
/// are separate dials and this is the second one.
fn band() -> usize {
    std::env::var("PACK_BAND").ok().and_then(|v| v.parse().ok()).unwrap_or(0)
}

/// How many items one slot may hold.
///
/// A player's finished board carries twelve or thirteen across all five slots.
/// Four to a slot is twenty, which is more items than the game can hand
/// anybody, and every one of them acts on its own cooldown.
fn per_slot() -> usize {
    std::env::var("PACK_ITEMS").ok().and_then(|v| v.parse().ok()).unwrap_or(99)
}


// ------------------------------------------------------------------ themes
//
// `design/monster-themes.md` is the argument; this is the table. A theme names
// two slots and a vocabulary, and the packer considers nothing outside them -
// which is what makes a creature legible in the first three seconds of a fight
// and what cuts the candidate pool by roughly sixty percent.

#[derive(Copy, Clone, PartialEq, Debug)]
enum Theme {
    Striker,
    Wall,
    Burner,
    Slower,
    Drainer,
    Caster,
}

impl Theme {
    /// The two grids this creature fills. Two rather than five: two is what a
    /// player can read at a glance, and five is what made every creature on
    /// the ladder the same creature.
    fn slots(self) -> &'static [SlotKind] {
        match self {
            Theme::Striker => &[SlotKind::Weapon, SlotKind::Gloves],
            Theme::Wall => &[SlotKind::Chest, SlotKind::Helmet],
            Theme::Burner => &[SlotKind::Weapon, SlotKind::Greaves],
            Theme::Slower => &[SlotKind::Greaves, SlotKind::Gloves],
            Theme::Drainer => &[SlotKind::Gloves, SlotKind::Helmet],
            Theme::Caster => &[SlotKind::Weapon, SlotKind::Helmet],
        }
    }

    /// Does this piece speak the theme's language?
    ///
    /// A piece that carries nothing either way - a plain frame, a bare
    /// material - is allowed everywhere: a board needs cores and filler to
    /// assemble at all, and refusing them would leave most themes unable to
    /// finish a single item.
    fn allows(self, d: &PieceDef) -> bool {
        use gearmaster_engine::curse::CurseKind;
        use gearmaster_engine::piece::{Action, Trigger};
        let says = |want: fn(&Action) -> bool| {
            d.triggers.iter().any(|t| match t {
                Trigger::OnActivate(a)
                | Trigger::OnBattleStart(a)
                | Trigger::OnAdjacentActivate(a)
                | Trigger::OnAlignedActivate(a)
                | Trigger::OnDiagonalActivate(a)
                | Trigger::OnOtherCast(a) => want(a),
                Trigger::Watch { then, .. } => want(then),
                Trigger::PerAdjacentItem { action, .. } => want(action),
                Trigger::Consume { per, .. } => want(per),
                Trigger::SpendGold { on_success, .. } => want(on_success),
                Trigger::SpendMana { on_success, on_failure, .. }
                | Trigger::Spend { on_success, on_failure, .. } => {
                    want(on_success) || want(on_failure)
                }
                Trigger::PerAdjacentEmpty(_) => false,
            })
        };
        let b = &d.base;
        let speaks = match self {
            Theme::Striker => {
                b.physical_damage != 0
                    || b.magic_damage != 0
                    || b.strength != 0
                    || says(|a| matches!(a, Action::Damage { .. }))
            }
            Theme::Wall => {
                b.armor != 0
                    || b.health != 0
                    || b.physical_harden != 0
                    || b.magic_harden != 0
                    || b.reflect != 0
                    || says(|a| matches!(a, Action::GainArmor(_) | Action::Grow(_)))
            }
            Theme::Burner => {
                b.physical_damage != 0
                    || b.magic_damage != 0
                    || says(|a| {
                        matches!(a, Action::Damage { .. })
                            || matches!(a, Action::Curse { kind: CurseKind::Searing, .. })
                    })
            }
            Theme::Slower => {
                d.speed_bonus != 0
                    || b.curse_resist != 0
                    || d.triggers.iter().any(|t| matches!(t, Trigger::OnBattleStart(_)))
                    || says(|a| {
                        matches!(
                            a,
                            Action::Curse {
                                kind: CurseKind::Frost | CurseKind::Stun | CurseKind::Misfire,
                                ..
                            }
                        ) || matches!(a, Action::ReduceCooldown(_))
                    })
            }
            Theme::Drainer => {
                b.mind != 0
                    || b.mind_resist != 0
                    || says(|a| {
                        matches!(
                            a,
                            Action::Drain { .. }
                                | Action::MindDamage { .. }
                                | Action::StunStrongest { .. }
                        )
                    })
            }
            Theme::Caster => {
                d.power_bonus != 0
                    || b.mana != 0
                    || matches!(
                        d.kind,
                        PieceKind::Spell | PieceKind::Ink | PieceKind::Orb | PieceKind::Book
                            | PieceKind::Alignment
                    )
                    || says(|a| {
                        matches!(a, Action::GainMana(_) | Action::GainForking(_))
                            | matches!(a, Action::GainEmpowerment(_) | Action::GainShield(_))
                    })
            }
        };
        speaks || plain(d)
    }
}

/// Carries nothing any theme would recognise. Cores and filler, which every
/// board needs whatever it is for.
fn plain(d: &PieceDef) -> bool {
    d.triggers.is_empty()
        && d.effect.is_none()
        && d.base.physical_damage == 0
        && d.base.magic_damage == 0
        && d.base.armor == 0
        && d.base.health == 0
}

/// The clusters, from `design/monster-themes.md`. Stretches rather than a
/// rotation, so a player has time to work out what is in front of them - and
/// ordered to teach: hit first, then that hitting is not enough. Rungs 45 and
/// beyond are deliberately unthemed.
fn theme_for(rung: usize) -> Option<Theme> {
    Some(match rung + 1 {
        1..=6 => Theme::Striker,
        7..=13 => Theme::Wall,
        14..=20 => Theme::Burner,
        21..=28 => Theme::Slower,
        29..=36 => Theme::Caster,
        37..=44 => Theme::Drainer,
        _ => return None,
    })
}

/// What this creature draws from. A mini-boss is a hybrid: its own cluster's
/// theme and the one the next cluster introduces, so it is both a harder
/// version of what you have learned and the first sight of what is coming.
fn themes_of(rung: usize, ordinary: bool) -> Vec<Theme> {
    let mut out: Vec<Theme> = theme_for(rung).into_iter().collect();
    if !ordinary {
        // The next cluster's theme, found by walking forward to the first rung
        // that answers differently.
        if let Some(next) = (rung + 1..50).find_map(|r| {
            theme_for(r).filter(|t| Some(*t) != theme_for(rung))
        }) {
            out.push(next);
        }
    }
    out
}

/// The creature being packed, and where it stands.
fn subject() -> (usize, bool) {
    use gearmaster_engine::combat::{Rank, LADDER};
    LADDER
        .iter()
        .position(|m| m.name == who())
        .map(|i| (i, LADDER[i].rank == Rank::Ordinary))
        .unwrap_or((0, true))
}

/// Pieces of one kind that may go in one slot, best first.
fn pool(slot: SlotKind, kind: PieceKind) -> Vec<usize> {
    let mut v: Vec<usize> = (0..CATALOG.len())
        .filter(|&i| {
            let d = &CATALOG[i];
            let (rung, ordinary) = subject();
            let themes = themes_of(rung, ordinary);
            // No theme - the run-in, or a creature off the ladder - means the
            // old behaviour: everything that fits the slot.
            let fits_theme = themes.is_empty() || themes.iter().any(|t| t.allows(d));
            d.kind == kind && d.slots().contains(&slot) && fits_theme
        })
        // Quest rewards are the far side of somebody's quest and are not gear
        // anybody wears.
        .filter(|&i| !gearmaster_engine::piece::is_quest_reward(CATALOG[i].name))
        // A boss trophy belongs to exactly one creature - it is the thing that
        // creature leaves behind - so Francis may wear his coat and nobody
        // else's. `boss_gear_belongs_to_exactly_one_monster` is the test that
        // catches this, and it caught it.
        .filter(|&i| !is_boss_only(CATALOG[i].name) || CATALOG[i].name == mine())
        .collect();
    v.sort_by_key(|&i| std::cmp::Reverse(piece_rating(&CATALOG[i])));
    // Skip the top of the order, but never empty the pool: a kind with three
    // entries still has to produce one.
    let skip = band().min(v.len().saturating_sub(1));
    v.split_off(skip)
}

/// One attempt at an item: concrete pieces for one recipe, strongest first
/// with a bit of jitter so repeated trials explore.
fn choose(slot: SlotKind, recipe: &[(PieceKind, usize, usize)], rng: &mut Rng) -> Vec<usize> {
    let mut out = Vec::new();
    for &(kind, min, max) in recipe {
        let p = pool(slot, kind);
        if p.is_empty() {
            continue;
        }
        // Take the minimum always, and sometimes reach for the optional extra:
        // a fuller item covers more cells, which is the whole objective.
        let want = if max > min && rng.below(3) > 0 { max } else { min };
        for k in 0..want {
            // Mostly the best of its kind, occasionally the next few down, so
            // the search is not one deterministic board.
            let span = p.len().min(6);
            let pick = if rng.below(4) == 0 { rng.below(span) } else { k.min(span - 1) };
            out.push(p[pick]);
        }
    }
    out
}

/// Every cell a piece would occupy at `(x, y)` in some rotation, or `None`.
fn footprint(reg: &PieceRegistry, id: gearmaster_engine::piece::PieceId, x: u8, y: u8, rows: u8)
    -> Option<Vec<(u8, u8)>>
{
    let mut cells = Vec::new();
    for &(dx, dy) in reg.shape(id).cells() {
        let (cx, cy) = (x as i32 + dx as i32, y as i32 + dy as i32);
        if cx < 0 || cy < 0 || cx >= SLOT_W as i32 || cy >= rows as i32 {
            return None;
        }
        cells.push((cx as u8, cy as u8));
    }
    Some(cells)
}

/// Try to seat one whole item, every piece of it touching the rest.
///
/// Returns the placements on success and leaves the board untouched on
/// failure, so a caller can simply try the next recipe.
#[allow(clippy::type_complexity)]
fn seat_item(
    reg: &mut PieceRegistry,
    lo: &mut Loadout,
    slot: SlotKind,
    defs: &[usize],
    rng: &mut Rng,
) -> Option<Vec<(&'static str, u8, u8, u8)>> {
    let rows = lo.slot(slot).rows();
    let ids: Vec<_> = defs.iter().map(|&d| reg.alloc(d)).collect();
    let mut placed: Vec<(gearmaster_engine::piece::PieceId, u8, u8, u8)> = Vec::new();

    for (n, &id) in ids.iter().enumerate() {
        let mut best: Option<(u8, u8, u8, usize)> = None;
        let mut order: Vec<(u8, u8)> =
            (0..rows).flat_map(|y| (0..SLOT_W).map(move |x| (x, y))).collect();
        if rng.below(2) == 0 {
            order.reverse();
        }
        for (x, y) in order {
            for rot in 0..4u8 {
                reg.set_rotation(id, rot);
                let Some(cells) = footprint(reg, id, x, y, rows) else { continue };
                if lo.can_place(reg, id, slot, x, y).is_err() {
                    continue;
                }
                // After the first piece, everything must touch what is already
                // down, or the group splits and the item never assembles.
                if n > 0 {
                    let touching = cells.iter().any(|&(cx, cy)| {
                        placed.iter().any(|&(pid, px, py, prot)| {
                            reg.set_rotation(pid, prot);
                            footprint(reg, pid, px, py, rows).is_some_and(|f| {
                                f.iter().any(|&(fx, fy)| {
                                    (fx as i32 - cx as i32).abs()
                                        + (fy as i32 - cy as i32).abs()
                                        == 1
                                })
                            })
                        })
                    });
                    reg.set_rotation(id, rot);
                    if !touching {
                        continue;
                    }
                }
                // Prefer the placement that hugs the top-left, which is what
                // leaves the remaining space in one usable block.
                let cost = cells.iter().map(|&(cx, cy)| cy as usize * 8 + cx as usize).sum();
                if best.is_none_or(|(_, _, _, b)| cost < b) {
                    best = Some((x, y, rot, cost));
                }
            }
        }
        let Some((x, y, rot, _)) = best else {
            // Undo: this item cannot be finished here.
            for &(pid, _, _, _) in &placed {
                lo.remove_anywhere(pid);
            }
            return None;
        };
        reg.set_rotation(id, rot);
        lo.slot_mut(slot).place(reg, id, x, y);
        placed.push((id, x, y, rot));
    }

    // The engine's own opinion of whether that is an item.
    lock_assembled_in(lo, reg, slot);
    let report = lo.report(reg, slot);
    let mine: Vec<_> = ids.iter().copied().collect();
    let ok = report
        .items
        .iter()
        .any(|it| it.assembled && mine.iter().all(|id| it.pieces.contains(id)));
    if !ok {
        for &(pid, _, _, _) in &placed {
            lo.remove_anywhere(pid);
        }
        // Only this attempt's locks come off. It used to clear every lock in
        // the loadout and re-derive - which unlocked every item seated before
        // it, and an unlocked item is one a later placement can be absorbed
        // into. That is the whole reason to lock as you go: a locked item is
        // finished and cannot be joined, so the next one may be packed flush
        // against it. Clearing them all meant each failed attempt undid the
        // density of everything already on the board.
        let still: std::collections::HashSet<_> =
            SlotKind::ALL.iter().flat_map(|&k| lo.slot(k).pieces()).collect();
        lo.locks.retain(|l| l.pieces.iter().all(|p| still.contains(p)));
        return None;
    }
    Some(
        placed
            .iter()
            .map(|&(pid, x, y, rot)| (CATALOG[reg.def_index(pid)].name, x, y, rot))
            .collect(),
    )
}

/// What the fight is supposed to come to.
///
/// Francis is the last rung and he is optional, so the two finished boards are
/// the right yardstick: the owner's cleared the ladder and the friend's is the
/// stronger of the two. One step harder than he was means the strong board
/// still takes him at the lower settings and stops taking him at the top.
///
/// Tuning a knob and re-measuring did not work: the search is stochastic and
/// two runs at the same power band produced boards that differed by more than
/// the band did - one where the friend won all four settings and one where it
/// lost all four. Scoring the outcome directly is the only thing that aims.
/// Measured off the board the creature already has, rather than written down.
///
/// Francis had his profile stated by hand because he is the last rung and
/// somebody had to decide what beating him should mean. Every other creature
/// already has an answer - the one its current board gives - and repacking is
/// meant to make a board *denser*, not harder. So the target is whatever the
/// existing spec does against the two finished builds, and a repack is
/// accepted only if it lands on the same table.
///
/// That is what makes this mechanical rather than fifty-three tuning problems:
/// balance is preserved by construction, and `PACK_BAND` only has to be moved
/// How many pieces a creature on this rung carries.
///
/// `design/monster-themes.md` §4: density is the curve and the theme is the
/// character. One more piece a rung from a floor of three, so rung 1 is four,
/// rung 25 is twenty-eight and rung 50 is fifty-three. A wall and a striker on
/// the same rung carry the same count and look nothing alike.
///
/// This replaced a ceiling of "twice what it has, or eight more" - a bound
/// relative to the board being replaced, which was the right guard while the
/// job was densifying existing boards and the wrong one the moment the job
/// became authoring them to a curve. It is why Bog Toad, on rung two, came back
/// with fifteen pieces when the curve asks for five.
fn pieces_for(rung: usize) -> usize {
    // Flat across the casino's window, then the curve. The casino is earned by
    // a quick kill inside the first ten rungs, and a ladder that thickens from
    // rung one closes that door before a player can reach it - which is how
    // the first themed run took the casino test down with it. Below eleven a
    // creature carries four or five; from eleven the line climbs a piece a
    // rung, reaching fifty-three at rung fifty as it always did.
    if rung < 10 {
        4 + rung / 4
    } else {
        3 + rung
    }
}


/// What a fight on this rung should take.
///
/// The gate used to ask "is this the same fight the creature already gave",
/// which cannot accept a themed board: a theme changes what a creature is on
/// purpose, so sameness is the wrong question. This is the right one - is this
/// the right *difficulty* for where it stands - and it needs a curve to be
/// asked against.
///
/// Read off the owner's board at Medium: Medium is one times, and the owner's
/// is the only reference board that clears far enough to give a reading at
/// every rung. Rising four tenths of a second a rung, from a floor of 2.8s -
/// rung 1 is 3.2s, rung 25 is 12.4s, rung 50 is 22.4s. Its median across the 46
/// rungs it currently clears is 14.4s, so the line runs through roughly where
/// the game already sits rather than moving it somewhere new.
///
/// The floor was two seconds and is 2.8 because the first attempt said so:
/// rung two wanted 2.4s and the best themed board any search could find took
/// 3.2, because a striker at rung two cannot be built weaker than that with
/// gear that assembles. A curve whose bottom end nothing can reach is a curve
/// that rejects the whole early ladder.
///
/// Linear on purpose: the brief is that difficulty should scale roughly
/// linearly, and a curve nobody can predict from its own shape is one nobody
/// can author against.
fn target_ms(rung: usize) -> u32 {
    2_800 + 400 * rung as u32
}

/// How far off the curve a candidate lands, as a fraction. Zero is exact, and
/// a loss is infinitely far - a creature the reference board cannot beat is
/// not on the curve at all.
fn off_curve(owner_medium: Beat, rung: usize) -> f64 {
    if !owner_medium.won {
        return f64::MAX;
    }
    let want = target_ms(rung) as f64;
    (owner_medium.ms as f64 - want).abs() / want
}

/// How far off a board may land and still be accepted.
const BAND: f64 = 0.30;

/// One fight: who won, and how long it took.
///
/// Outcome alone is not enough and the ladder proved it. The preset board
/// already loses to a mid-rung creature on Insane; it still loses after a
/// repack, so a win-and-loss table came back unchanged while the fight behind
/// it had got materially harder - and the run that has to walk through that
/// rung to reach an event twelve rungs later stopped arriving. A board can get
/// much worse to fight without flipping a single bit.
#[derive(Copy, Clone, Default, PartialEq)]
struct Beat {
    won: bool,
    ms: u32,
}

#[allow(dead_code)]
impl Beat {
    /// Near enough the same fight. Within a quarter either way, which is the
    /// band `analysis/baseline.md` has been reading time-to-kill against since
    /// the baseline was captured.
    fn like(self, other: Beat) -> bool {
        if self.won != other.won {
            return false;
        }
        let (a, b) = (self.ms.max(1) as f64, other.ms.max(1) as f64);
        (a / b).max(b / a) <= 1.25
    }
}

#[allow(dead_code)]
fn want() -> Vec<[Beat; 4]> {
    use gearmaster_engine::combat::LADDER;
    let base = *LADDER.iter().find(|m| m.name == who()).expect("on the ladder");
    fight(base.gear, base.items)
}

/// Fight a candidate board with both finished builds.
fn fight(gear: &'static [(&'static str, SlotKind, u8, u8, u8)], chunks: &'static [usize])
    -> Vec<[Beat; 4]>
{
    use gearmaster_engine::combat::{simulate_at, Difficulty, Outcome, LADDER};
    let base = *LADDER.iter().find(|m| m.name == who()).expect("on the ladder");
    let spec = gearmaster_engine::combat::MonsterSpec { gear, items: chunks, ..base };
    boards()
        .iter()
        .map(|(_, run)| {
            let (st, items) = (run.player_stats(), run.combat_items());
            let mut row = [Beat::default(); 4];
            for (i, d) in Difficulty::ALL.iter().enumerate() {
                let log = simulate_at(st, &items, &spec, *d);
                row[i] = Beat { won: log.outcome == Outcome::Victory, ms: log.duration_ms };
            }
            row
        })
        .collect()
}

fn boards() -> Vec<(&'static str, gearmaster_engine::run::Run)> {
    use gearmaster_engine::run::{Mode, Run};
    use gearmaster_engine::share;
    // The two finished boards come back through the one reconstruction there
    // is. This function used to hand-roll the placement loop without locking
    // as it went, which is the fault `common::board_from` exists to end - and
    // it mattered here more than anywhere, because the curve every creature is
    // packed against is read off the owner's board.
    // The preset first, and it is the one that matters for the early ladder.
    // Two finished ladder-clearing boards beat a rung-two creature whatever it
    // is wearing, so scoring only against them left the search free to pack
    // Bog Toad to fifty-six pieces and call the profile unchanged - it *was*
    // unchanged, because neither yardstick could feel the difference. The
    // preset clears eleven rungs, which is roughly what a player has in hand
    // early, and it loses to an over-packed creature the moment one exists.
    [("preset", ""), ("owner", share::A_WINNING_RUN), ("friend", share::A_FRIENDS_RUN)]
        .into_iter()
        .map(|(label, code)| {
            if code.is_empty() {
                let mut r = Run::new();
                r.mode = Mode::Grinder;
                r.apply_preset();
                return (label, r);
            }
            (label, common::run_from(code))
        })
        .collect()
}

#[test]
#[ignore = "generator; run with --ignored"]
fn pack() {
    let mut best: Option<(usize, usize, String, Vec<usize>, Vec<[Beat; 4]>)> = None;

    for trial in 0..300u64 {
        let mut rng = Rng::new(0x5EED_0000 + trial);
        let mut lines: Vec<String> = Vec::new();
        let mut gear: Vec<(&'static str, SlotKind, u8, u8, u8)> = Vec::new();
        let mut chunks: Vec<usize> = Vec::new();
        let mut total = 0usize;

        // Only the theme's grids. A creature that fills all five is the
        // creature this design exists to stop being.
        let (rung, ordinary) = subject();
        let drawn = themes_of(rung, ordinary);
        let wanted: Vec<SlotKind> = if drawn.is_empty() {
            SlotKind::ALL.to_vec()
        } else {
            let mut v: Vec<SlotKind> = Vec::new();
            for t in &drawn {
                for s in t.slots() {
                    if !v.contains(s) {
                        v.push(*s);
                    }
                }
            }
            v
        };
        for slot in wanted {
            let mut reg = PieceRegistry::new();
            let mut lo = Loadout::new();
            let all = recipes(slot);
            // Francis swings. Left to itself the search handed him two orb
            // weapons casting three spells apiece, which put 4954 damage into
            // a 2680-health board before anything else had happened - and it
            // is also simply not him: he is a gambler in a coat with a sword.
            let recs: &[&[(PieceKind, usize, usize)]] =
                if slot == SlotKind::Weapon && who() == "Francis" { &all[..1] } else { all };
            // The coat goes on first. It is a Base, it is four cells by three,
            // and it is the one strange thing Francis owns - a board packed
            // around it is a different board from one packed without it, so it
            // cannot be left to whether the search happens to reach for it.
            let mut stalled = 0;
            let mut here = 0usize;
            if slot == SlotKind::Chest && !mine().is_empty() {
                let coat = CATALOG.iter().position(|d| d.name == mine()).expect("in the catalogue");
                let layer = pool(slot, PieceKind::Layer);
                for &l in layer.iter().take(4) {
                    if let Some(p) = seat_item(&mut reg, &mut lo, slot, &[coat, l], &mut rng) {
                        here += 1;
                        chunks.push(p.len());
                        for (name, x, y, rot) in p {
                            gear.push((name, slot, x, y, rot));
                            lines.push(format!(
                                "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
                                name, slot, x, y, rot
                            ));
                        }
                        break;
                    }
                }
            }
            // One weapon. A player carries one; a creature carrying three
            // swings three times a cooldown and no board can answer that.
            let cap = if slot == SlotKind::Weapon { 1 } else { per_slot() };
            // The ceiling is enforced here rather than on the finished
            // candidate: the loop fills a slot at a time, so a board that is
            // going to be too big is too big from early on, and rejecting it
            // afterwards simply threw every candidate away.
            let cap_here = pieces_for(subject().0);
            while stalled < 40 && here < cap && gear.len() < cap_here {
                let r = recs[rng.below(recs.len())];
                let defs = choose(slot, r, &mut rng);
                if defs.is_empty() {
                    stalled += 1;
                    continue;
                }
                match seat_item(&mut reg, &mut lo, slot, &defs, &mut rng) {
                    // An item that would carry the board past the curve is
                    // refused and its cells given back. Checking the bound
                    // before seating let a whole item through, which is nothing
                    // at rung thirty and forty percent at rung two - and rung
                    // two is where the early ladder can least afford it.
                    Some(p) if gear.len() + p.len() > cap_here => {
                        for (name, ..) in &p {
                            if let Some(id) = lo
                                .slot(slot)
                                .pieces()
                                .into_iter()
                                .find(|&q| reg.def(q).name == *name)
                            {
                                lo.slot_mut(slot).remove(id);
                            }
                        }
                        stalled += 1;
                    }
                    Some(p) => {
                        here += 1;
                        chunks.push(p.len());
                        for (name, x, y, rot) in p {
                            gear.push((name, slot, x, y, rot));
                            lines.push(format!(
                                "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
                                name, slot, x, y, rot
                            ));
                        }
                        stalled = 0;
                    }
                    None => stalled += 1,
                }
            }
            let s = lo.slot(slot);
            total += (0..s.rows())
                .flat_map(|y| (0..SLOT_W).map(move |x| (x, y)))
                .filter(|&(x, y)| s.get(x, y).is_some())
                .count();
        }

        // Leaked so the spec can borrow them for the length of the fight. This
        // is a generator that runs once by hand; the alternative is threading a
        // lifetime through `MonsterSpec` for the benefit of one test.
        let got = fight(Box::leak(gear.into_boxed_slice()), Box::leak(chunks.clone().into_boxed_slice()));
        // Boards are [preset, owner, friend]; difficulties [Easy, Medium,
        // Hard, Insane]. The owner at Medium is the reading.
        let (rung, _) = subject();
        let miss = off_curve(got[1][1], rung);
        // Whole percent off, inverted, so closer sorts higher and the tuple
        // below still compares cleanly against density.
        let hits = if miss == f64::MAX { 0 } else { 1000 - (miss * 1000.0).min(1000.0) as usize };
        // Outcome first, density second: a board that fights right at seventy
        // percent is worth more than one that fights wrong at ninety.
        let key = (hits, total);
        if best.as_ref().is_none_or(|(h, t, ..)| key > (*h, *t)) {
            best = Some((hits, total, lines.join("\n"), chunks, got));
        }
    }

    let (hits, total, out, chunks, got) = best.expect("something was packed");
    // A minimum bar, not just a ranking. The search takes the best candidate it
    // found, and "best" is not the same as "good enough": Rust Colossus came
    // back turning seven-second fights into forty-three-second stalemates and
    // still counted as the winner of its own trial set, because nothing closer
    // existed. Failing here rather than printing a board means the batch runner
    // records a skip and leaves the creature exactly as it was, which is the
    // right answer for any board this search cannot match.
    let (rung, _) = subject();
    let miss = off_curve(got[1][1], rung);
    assert!(
        miss <= BAND,
        "nothing landed on the curve for rung {}: wanted {:.1}s within {:.0}%, best was {}. \
         Leaving it alone.",
        rung + 1,
        target_ms(rung) as f64 / 1000.0,
        BAND * 100.0,
        if got[1][1].won {
            format!("{:.1}s", got[1][1].ms as f64 / 1000.0)
        } else {
            "a loss".into()
        },
    );
    let cap = SLOT_W as usize * SLOT_H as usize * 5;
    println!("BEST {total}/{cap} cells ({:.0}%), {hits}/8 outcomes on target", 100.0 * total as f32 / cap as f32);
    for (want, have) in want().iter().zip(&got) {
        let show = |r: &[Beat; 4]| {
            r.iter()
                .map(|b| format!("{}{:.1}s", if b.won { "W" } else { "L" }, b.ms as f64 / 1000.0))
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!("  board want {} got {}", show(want), show(have));
    }
    println!("GEAR");
    println!("{out}");
    println!("ITEMS &{chunks:?}");
    println!("pieces: {}", out.matches("\", SlotKind").count());
    let _ = is_boss_only("");
}
