//! A run, played forward.
//!
//! The first thing in this repo's history to walk the road with only the
//! actions a person has: a tray, five grids, a shelf, whatever is standing on
//! the rung, and the keys. No fight is simulated that the run is not standing
//! in, and nothing is read that the screen does not draw.
//!
//! ## The order it decides in
//!
//! Not a policy - a **priority**, which is what a control is. Whatever is in
//! front of the run is answered first, because the road stack pops in an order
//! and a door in front of another is not a queue (`CLAUDE.md` §6 trap 35).
//! Underneath that: keep the board packed, spend gold on what the board is
//! short of, and fight.
//!
//! A6 replaces every one of these rules with something learned. They are here
//! so that there is a number to beat, and so that the number was produced by
//! playing.

use crate::hands;
use crate::seen::Seen;
use gearmaster_console::{Console, Difficulty, Door, Mode, PieceKind, Verb};

/// What the pilot is playing for.
///
/// One dial today and two more at A5 and A8. `patience` is how many fights it
/// will spend on a rung it is not getting past - a Grinder may farm for ever
/// and the budget is otherwise the only thing that stops it.
#[derive(Copy, Clone, Debug)]
pub struct Doctrine {
    pub patience: usize,
    /// Presses. The whole budget for the run.
    pub budget: usize,
    /// Door-seeking against rung-seeking, zero to one.
    ///
    /// At zero the pilot takes the first open choice, which is what A4
    /// measured. Above zero it prefers a branch **nothing has taken before**,
    /// across every run in the sweep - which is what turns a clear rate into
    /// coverage. It is a dial rather than a switch because the two objectives
    /// genuinely trade: a door that costs a life is a door a win-seeker will
    /// not open and a ledger needs opened.
    pub coverage: f32,
}

impl Default for Doctrine {
    fn default() -> Self {
        // Twenty-four rather than eight: A6's plateau test found three times
        // the patience buys a great deal and ten times buys nothing, so this
        // is the knee of that curve rather than a round number.
        Doctrine { patience: 24, budget: 600_000, coverage: 0.0 }
    }
}

/// How a run ended, and what it did on the way.
#[derive(Clone, Debug, PartialEq)]
pub struct Ended {
    pub seed: u64,
    pub best_rung: usize,
    pub board_clears: usize,
    pub game_clears: usize,
    pub losses: usize,
    pub presses: usize,
    pub bought: usize,
    pub sold: usize,
    pub bartered: usize,
    pub rerolled: usize,
    pub grew: usize,
    pub cleared: usize,
    /// Times a door put this run inside a dungeon.
    pub dungeons: usize,
    /// Orbs fed to a pedestal.
    pub orbs: usize,
    pub doors: usize,
    pub towns: usize,
    pub why: &'static str,
    /// The door a run could not get past, if that is what stopped it.
    pub stuck_at: Option<String>,
    /// The narrowest loss the run suffered, as the share of the creature's
    /// health still standing when the run went down. Zero would be a kill.
    ///
    /// This is the figure that decides whether a plateau is **evaluation** or
    /// **exploration** (the spec's A4): a run losing by five percent is one
    /// whose board is nearly right, and a run losing by seventy is one whose
    /// tray never held the family the fight wanted.
    pub narrowest_loss: Option<f64>,
    /// Every verb pressed, so a clear can be written out as a transcript.
    pub transcript: Vec<String>,
}

/// Play one seed as far as it goes, remembering nothing.
pub fn play(seed: u64, mode: Mode, difficulty: Difficulty, d: Doctrine) -> Ended {
    let mut nothing = Seen::default();
    play_remembering(seed, mode, difficulty, d, &mut nothing)
}

/// Play one seed with a learned prior ranking the seats.
pub fn play_guided(
    seed: u64,
    mode: Mode,
    difficulty: Difficulty,
    d: Doctrine,
    seen: &mut Seen,
    prior: &dyn crate::hands::Prior,
) -> Ended {
    play_impl(seed, mode, difficulty, d, seen, Some(prior))
}

/// Play one seed, writing what it meets into a memory that outlives it.
///
/// The memory is what makes the coverage dial mean anything: a run that
/// prefers an untaken branch has to know which branches other runs took.
pub fn play_remembering(
    seed: u64,
    mode: Mode,
    difficulty: Difficulty,
    d: Doctrine,
    seen: &mut Seen,
) -> Ended {
    play_impl(seed, mode, difficulty, d, seen, None)
}

fn play_impl(
    seed: u64,
    mode: Mode,
    difficulty: Difficulty,
    d: Doctrine,
    seen: &mut Seen,
    prior: Option<&dyn crate::hands::Prior>,
) -> Ended {
    let mut c = Console::start(seed, mode, difficulty);
    seen.runs += 1;
    let mut e = Ended {
        seed,
        best_rung: 1,
        board_clears: 0,
        game_clears: 0,
        losses: 0,
        presses: 0,
        bought: 0,
        sold: 0,
        bartered: 0,
        rerolled: 0,
        grew: 0,
        cleared: 0,
        dungeons: 0,
        orbs: 0,
        doors: 0,
        towns: 0,
        why: "out of presses",
        stuck_at: None,
        narrowest_loss: None,
        transcript: Vec::new(),
    };
    let mut stuck = 0usize;
    let mut tray_changed = true;
    // Shop presses since the last fight.
    //
    // A reroll is a coin against six new shelves and a run with six hundred
    // gold can afford six hundred of them, so without a bound the shop is an
    // infinite loop that eats the whole press budget - which is exactly what
    // it did on the first run of this, at ten minutes and counting.
    let mut shopping = 0usize;
    const SHOP_PRESSES: usize = 24;
    // Set by a lost fight, spent by one rebuild, and **at most once a rung**.
    //
    // A rebuild is a whole re-pack - fifteen thousand presses on a full board -
    // and patience is twenty-four, so "rearrange after every loss" is
    // twenty-four rebuilds a rung and a budget gone. Once is a player
    // rearranging after a defeat; twenty-four times is a search, and this
    // pilot is not one.
    let mut rearrange = false;
    let mut rearranged_here = false;
    // Once a run each, not once a press: a Grinder farming rung 12 stood on
    // it once as far as coverage is concerned.
    let mut reached_this_run: std::collections::BTreeSet<usize> = Default::default();

    while e.presses < d.budget && !c.over() {
        let menu = c.menu();
        if menu.is_empty() {
            e.why = "nothing left to press";
            break;
        }
        let v = c.view();

        // Everything this run is standing in, recorded before it decides.
        seen.deepest_rung = seen.deepest_rung.max(v.rung_shown);
        if reached_this_run.insert(v.rung_shown) {
            *seen.rungs_stood.entry(v.rung_shown).or_default() += 1;
        }
        if let Some(d) = &v.dungeon {
            seen.floors.entry(d.id.clone()).or_default().insert(d.at);
        }
        if let Some(cy) = &v.county {
            seen.county_tiles.insert(cy.reference.clone());
        }
        for name in &v.classes {
            seen.classes.insert(name.clone());
        }

        // ---- whatever is in front of the run --------------------------
        let was_inside = v.dungeon.is_some();
        if let Some(q) = &v.question {
            // Offered: this run stood in front of it, on this rung.
            seen.doors_offered.entry(q.id.clone()).or_default().insert(v.rung_shown);
            let open: Vec<usize> = q.choices.iter().filter(|c| c.open).map(|c| c.index).collect();
            // What the shut choices are waiting for. A requirement of the form
            // `Requires: having chosen "X" earlier` names a choice by label,
            // and the interface prints it under the greyed button - so this is
            // reading the screen rather than the rules.
            for c in q.choices.iter().filter(|c| !c.open) {
                if let Some(rest) = c.requires.split_once('"') {
                    if let Some((label, _)) = rest.1.split_once('"') {
                        seen.wanted_labels.insert(label.to_string());
                    }
                }
            }
            seen.choices_open.entry(q.id.clone()).or_default().extend(open.iter().copied());

            // Which branch.
            //
            // **A door that leads into a dungeon is taken first**, if the run
            // has learned that it does and has not already walked that dungeon
            // out. Nothing else gets a pilot underground: there is no `Enter`
            // verb because there is no such player action, so twenty-four
            // floors across seven dungeons sit entirely behind this choice.
            let into = open.iter().copied().find(|&i| {
                seen.leads_into(&q.id, i).is_some_and(|dg| !seen.walked_out(dg, floors_of(dg)))
            });
            // Then a choice some shut door has asked for by name. This is the
            // only thing that reaches across doors, and it is what a chain is:
            // the crevice's own door wants a deal taken two doors earlier, and
            // nothing at the deal says so.
            let asked_for = into.or_else(|| {
                open.iter().copied().find(|&i| {
                    q.choices.get(i).is_some_and(|c| seen.is_wanted(&c.label))
                })
            });
            // Otherwise the least-taken open one. A pilot that always takes
            // the first can never find out what the second does, and finding
            // out is the only way the line above ever has anything to say.
            let want =
                asked_for.or_else(|| open.iter().copied().min_by_key(|&i| seen.times(&q.id, i)));
            let pick = want
                .map(|choice| Verb::Answer { choice })
                .filter(|p| menu.contains(p))
                .or_else(|| menu.iter().find(|x| matches!(x, Verb::Answer { .. })).copied())
                .or_else(|| menu.iter().find(|x| matches!(x, Verb::AnswerWith { .. })).copied());
            let Some(pick) = pick else {
                // A door with nothing open under it. Nothing else will be
                // offered until it is answered, so this is where a run stops -
                // which is a finding rather than a bug.
                e.why = "a door with no open choice";
                e.stuck_at = Some(q.id.clone());
                break;
            };
            if !press(&mut c, pick, &mut e) {
                break;
            }
            if let Verb::Answer { choice } | Verb::AnswerWith { choice, .. } = pick {
                *seen.choices_taken.entry(q.id.clone()).or_default().entry(choice).or_default() +=
                    1;
                // Where it put us. Checked after the press rather than read off
                // the choice, because the pilot cannot see what an outcome is -
                // only where it ends up standing.
                if let Some(dg) = c.view().dungeon.as_ref().map(|dg| dg.id.clone()) {
                    if !was_inside {
                        seen.doors_into.insert((q.id.clone(), choice), dg);
                        e.dungeons += 1;
                    }
                }
            }
            e.doors += 1;
            continue;
        }

        if let Some(town) = &v.town {
            // One action a visit. A shop when there is room to carry
            // something, a shift at the works when there is not, and the gate
            // itself when neither is on offer.
            // At coverage the gate is a door-shaped opportunity rather than a
            // shop: take one this town has never been through. That is what
            // gets a run down the steps into THE HUNDRED, which the first
            // sweep never once did - not because the county is unreachable
            // but because the pilot always wanted the shop.
            let door = if d.coverage > 0.0 {
                town.doors.iter().map(|(dr, _)| *dr).min_by_key(|dr| {
                    seen.town_times(&town.name, &format!("{:?}", dr).to_lowercase())
                })
            } else {
                // The pub is on this list because of what A5 measured and A6
                // half-fixed: six road doors want a rumour, a rumour is
                // bartered for at a pub, and a win-seeking pilot that only
                // ever wanted the shop never went to one. Bartering appeared
                // in the coverage sweep and nowhere else, and that was why.
                [Door::Shop, Door::Pub, Door::Factory, Door::Chapel]
                    .into_iter()
                    .filter(|dr| *dr != Door::Shop || v.tray.len() + 2 < v.tray_cap)
                    .find(|dr| town.doors.iter().any(|(have, _)| have == dr))
            };
            let pick = match door {
                Some(dr) if menu.contains(&Verb::Town { door: dr }) => Verb::Town { door: dr },
                _ => Verb::WalkOn,
            };
            seen.gates.insert(town.name.clone());
            if !press(&mut c, pick, &mut e) {
                break;
            }
            if let Verb::Town { door } = pick {
                *seen
                    .town_doors
                    .entry(town.name.clone())
                    .or_default()
                    .entry(format!("{:?}", door).to_lowercase())
                    .or_default() += 1;
            }
            e.towns += 1;
            tray_changed = true;
            continue;
        }

        if v.fountain.is_some() {
            // Three verbs, not one. The third fountain doubles a class you
            // already hold and offers `Double` alone - no `Drink` at all - so
            // a pilot that looks only for a drink stops dead in front of it.
            // Seed 0x1212 reached rung 47 and was stopped by this rather than
            // by the game.
            let pick = menu
                .iter()
                .find(|x| matches!(x, Verb::DrinkChoosing { .. }))
                .or_else(|| menu.iter().find(|x| matches!(x, Verb::Double { .. })))
                .or_else(|| menu.iter().find(|x| matches!(x, Verb::Drink)))
                .copied();
            let Some(pick) = pick else {
                e.why = "a fountain offering nothing";
                break;
            };
            if !press(&mut c, pick, &mut e) {
                break;
            }
            continue;
        }

        // ---- THE HUNDRED ------------------------------------------------
        //
        // A run standing in the county is offered `Walk` and `Out` and nothing
        // else, so a pilot with no branch for it reaches "nothing worth
        // pressing" and the run ends on the steps. The first sweep never went
        // down, so the stall was never met - a gap that hides behind another
        // gap is the shape this mission keeps finding.
        if let Some(county) = &v.county {
            // Toward a tile nothing has cleared, and only then any tile at
            // all. A step into the pale costs the move and teaches the run
            // where the pale is, which is why it is not refused here.
            let unwalked = county
                .around
                .iter()
                .filter_map(|(key, n)| n.as_ref().map(|n| (key, n)))
                .filter(|(_, n)| !n.cleared && !n.sealed)
                .map(|(key, _)| key.clone())
                .next();
            let step = unwalked
                .or_else(|| {
                    county.around.iter().find(|(_, n)| n.is_some()).map(|(k, _)| k.clone())
                })
                .and_then(|k| gearmaster_console::Step::parse(&k));
            let pick = step
                .map(|step| Verb::Walk { step })
                .filter(|p| menu.contains(p))
                .unwrap_or(Verb::Out);
            if !press(&mut c, pick, &mut e) {
                break;
            }
            continue;
        }

        if let Some(points) = &v.points {
            // A set of points is a question about a graph, and since THE ATLAS
            // the map draws the graph - so take a road that has not been
            // walked, and only then one that has.
            let fresh = points.exits.iter().find(|(_, _, _, cleared)| !cleared).map(|(i, ..)| *i);
            let exit = fresh.or_else(|| points.exits.first().map(|(i, ..)| *i));
            let pick = exit
                .map(|exit| Verb::ThrowPoints { exit })
                .filter(|p| menu.contains(p))
                .unwrap_or(Verb::Leave);
            if !press(&mut c, pick, &mut e) {
                break;
            }
            continue;
        }

        // ---- the board -------------------------------------------------
        if tray_changed && !v.tray.is_empty() {
            let room = d.budget.saturating_sub(e.presses);
            let mut packed = match prior {
                Some(p) => hands::pack_with(&mut c, room, p),
                None => hands::pack(&mut c, room),
            };
            // Anything the hands would not seat is a piece with nowhere to go,
            // and on a full board that is every piece. Take a grid apart and
            // rebuild it from everything available.
            // Only after a loss, and only once for it. Taking a grid apart
            // and rebuilding it costs thousands of presses; doing it every
            // rung took the median run from 34 to 10 and every seed ran out of
            // budget. It is what a person does when a fight goes badly - lose,
            // rearrange, try again - rather than a thing done routinely.
            if packed.left_in_tray > 0 && rearrange && !rearranged_here {
                rearrange = false;
                rearranged_here = true;
                let again = hands::reseat_with(
                    &mut c,
                    d.budget.saturating_sub(e.presses + packed.presses),
                    prior,
                );
                packed.presses += again.presses;
                packed.seated += again.seated;
                packed.cleared += again.cleared;
            }
            e.cleared += packed.cleared;
            e.presses += packed.presses;
            for verb in c.history().iter().skip(e.transcript.len()) {
                e.transcript.push(c.annotate(*verb));
            }
            tray_changed = false;
            continue;
        }

        // An orb in the tray is a place the run has not been.
        //
        // Three of the seven dungeons - the undertow, den rivals and wumpus
        // world - have **no road in at all**: an Orb of Travel fed to a
        // pedestal is the only way, and six destinations hang off the same
        // verb. The pilot has had `Pedestal` since A1 and never once pressed
        // it, which is the fifth verb this mission has found in that state.
        if let Some(pick) = menu.iter().find(|x| matches!(x, Verb::Pedestal { .. })).copied() {
            if press(&mut c, pick, &mut e) {
                e.orbs += 1;
                continue;
            }
        }

        // A row that has been granted and not spent is six cells nobody has.
        // `Grow` is one of the four verbs no interface had before A1.
        if let Some(pick) = menu.iter().find(|x| matches!(x, Verb::Grow { .. })).copied() {
            if press(&mut c, pick, &mut e) {
                e.grew += 1;
                tray_changed = true;
                continue;
            }
        }

        // ---- the three verbs the pilot owned and never pressed ----------
        //
        // A5 found the first: six road doors want a rumour, and a rumour is
        // **bartered** for rather than bought. A6 found the other two by
        // asking why a run sits on six hundred gold at the wall with seven
        // items - it buys one piece a rung by a crude rule, cannot sell what
        // will not sit, and never rerolls a shelf it does not want.
        //
        // A barter first, because a rumour is the only thing on a shelf that
        // opens a door rather than filling a cell.
        if let Some(pick) = menu
            .iter()
            .find(|x| matches!(x, Verb::Barter { .. }))
            .copied()
            .filter(|_| shopping < SHOP_PRESSES && (v.tray.len() > 1 || d.coverage > 0.0))
        {
            shopping += 1;
            if press(&mut c, pick, &mut e) {
                e.bartered += 1;
                tray_changed = true;
                continue;
            }
        }

        // Then room to carry the next thing. A piece the hands would not seat
        // is a piece that is paying nothing for its place in the tray.
        if v.tray.len() + 1 >= v.tray_cap && shopping < SHOP_PRESSES {
            shopping += 1;
            // **Never a word.** Every rumour in the game costs one gold - the
            // cheapest thing in five hundred and twenty-three pieces - so
            // "sell the cheapest" sold the key the pilot had just bartered
            // for, every single time the tray filled. Forty barters across six
            // runs and not one rumour still held at the end.
            //
            // `PieceKind::Quest` is what the card says it is, so this is a
            // thing the pilot can see rather than a name it was told.
            let worst = v
                .tray
                .iter()
                .filter(|p| p.kind != PieceKind::Quest)
                .filter_map(|p| p.id.map(|id| (p.price, id)))
                .min_by_key(|(price, _)| *price)
                .map(|(_, id)| id);
            if let Some(piece) = worst {
                let pick = Verb::Sell { piece };
                if menu.contains(&pick) && press(&mut c, pick, &mut e) {
                    e.sold += 1;
                    continue;
                }
            }
        }

        // ---- the shelf --------------------------------------------------
        //
        // One-step lookahead with the real shop: buy the piece whose slot has
        // the least finished, while there is room to carry it and gold to
        // spare for the next rung. Not a value-of-information search - that
        // wants a packer in the loop and the packer is the expensive thing.
        if let Some(shelf) = want_to_buy(&v).filter(|_| shopping < SHOP_PRESSES) {
            shopping += 1;
            let pick = Verb::Buy { shelf };
            if menu.contains(&pick) && press(&mut c, pick, &mut e) {
                e.bought += 1;
                tray_changed = true;
                continue;
            }
        }

        // Nothing on the shelf was worth carrying. A reroll is a coin against
        // a whole new set of six, and a run standing on six hundred gold has
        // no better use for one.
        // Only while hoarding. `gold > reroll_cost * 8` was true at rung two
        // with thirty coins, so a run rerolled six times before it had
        // anything to spend on and arrived at its first real fight poorer than
        // it started - seed 0x1212 went from rung 51 to rung 2 on that alone.
        // The finding this verb came from was a run sitting on six hundred
        // gold, and that is the condition it belongs under.
        if shopping < SHOP_PRESSES && hoarding(&v) && menu.contains(&Verb::Reroll) {
            shopping += 1;
            if press(&mut c, Verb::Reroll, &mut e) {
                e.rerolled += 1;
                continue;
            }
        }

        // ---- the fight ---------------------------------------------------
        let pick = menu
            .iter()
            .find(|x| matches!(x, Verb::FightParty))
            .or_else(|| menu.iter().find(|x| matches!(x, Verb::Fight)))
            .or_else(|| menu.iter().find(|x| matches!(x, Verb::Leave)))
            .copied();
        let Some(pick) = pick else {
            e.why = "nothing worth pressing";
            break;
        };
        if !press(&mut c, pick, &mut e) {
            break;
        }

        if matches!(pick, Verb::FightParty) {
            seen.brawls += 1;
        }
        if matches!(pick, Verb::Fight | Verb::FightParty) {
            shopping = 0;
        }
        if matches!(pick, Verb::Fight | Verb::FightParty) {
            let after = c.view();
            if let Some(f) = &after.last_fight {
                if f.won {
                    e.game_clears += 1;
                    if f.board_decided {
                        e.board_clears += 1;
                    }
                } else {
                    e.losses += 1;
                    rearrange = true;
                    // How much of the creature was left standing. The view's
                    // `coming` is the same creature, and its stats are the
                    // maximum - the card draws both.
                    let max = after.coming.stats.health.max(1) as f64;
                    let left = f.enemy_health_left.max(0) as f64 / max;
                    e.narrowest_loss =
                        Some(e.narrowest_loss.map_or(left, |b: f64| b.min(left)));
                }
            }
            if after.rung_shown > e.best_rung {
                e.best_rung = after.rung_shown;
                stuck = 0;
                rearranged_here = false;
            } else {
                stuck += 1;
                if stuck >= d.patience {
                    e.why = "stuck below its ceiling";
                    break;
                }
            }
        }
    }

    if c.over() {
        e.why = "the run ended";
    }
    e
}

/// How many floors a dungeon has.
///
/// Read off the atlas, which draws every floor of a dungeon a run has been
/// into (`console/src/view.rs`). Before a run has been in one it does not know
/// the size, and `walked_out` is false for an unknown dungeon - which is the
/// right answer, because a dungeon it has never seen is one it has not walked
/// out of.
fn floors_of(_dungeon: &str) -> usize {
    // A conservative floor count: the largest any dungeon has is nine, and
    // treating every one as nine means the pilot keeps going back until it has
    // actually seen nine floors of it. `Seen::floors` is what stops that being
    // for ever - it only counts distinct floors stood on.
    9
}

/// Which shelf to take, if any.
///
/// The board is five grids and a piece belongs to one of them, so the question
/// is which grid is least finished - a slot with no item at all is worth more
/// than a fourth ring for one that has three.
///
/// Public because a road agent's `Pack` is buy-and-seat rather than seat, and
/// the buying half is this. Reused rather than rewritten so "the control" means
/// one thing: `crates/lab/src/packers.rs` calls this and `hands::pack`, which
/// between them are the quartermaster half of the pilot every benchmark in
/// `analysis/the-two-trades.md` was measured against.
pub fn want_to_buy(v: &gearmaster_console::View) -> Option<usize> {
    // Keep a rung's bounty in hand. A run that spends to the last coin cannot
    // reroll, cannot barter and cannot pay a toll.
    let spare = v.gold - 8;
    if spare <= 0 || v.tray.len() + 1 >= v.tray_cap {
        return None;
    }
    let finished = |slot| {
        v.grids
            .iter()
            .find(|g| g.slot == slot)
            .map(|g| g.items.iter().filter(|i| i.assembled).count())
            .unwrap_or(0)
    };
    v.shop
        .iter()
        .filter(|s| s.price.is_some_and(|p| p <= spare))
        .min_by_key(|s| (finished(s.piece.slot), std::cmp::Reverse(s.piece.cells)))
        .map(|s| s.index)
}

/// Whether the run has more gold than it has any other use for.
///
/// A6's measurement: a run stuck at the wall carries five to eight hundred
/// gold and seven items. Gold that never becomes a board is a rung that never
/// gets cleared.
pub fn hoarding(v: &gearmaster_console::View) -> bool {
    v.gold > 200
}

fn press(c: &mut Console, v: Verb, e: &mut Ended) -> bool {
    let line = c.annotate(v);
    let out = c.apply(v);
    e.presses += 1;
    if out.ok {
        e.transcript.push(line);
    }
    out.ok
}

/// Seed-clear: did this run reach the rung, board-decided all the way?
pub fn reached(e: &Ended, rung: usize) -> bool {
    e.best_rung > rung
}
