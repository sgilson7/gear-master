//! A written road policy that follows a plan, for the composition to be
//! measured against.
//!
//! Not a baseline. **An upper bound.** This one is handed the chain and told
//! which choice at each door passes which stop, so it answers a different
//! question from the one a learned pathfinder answers: *is this chain reachable
//! at all by a run that packs with the control and shops off the list?* If the
//! answer is no, no amount of training will make it yes and the fault is
//! somewhere else.
//!
//! The learned policy's honest baseline is the other control in `qaim` - the
//! first legal step every time - which knows nothing.
//!
//! It is a road policy only. The packing and the shopping are the same two
//! things a trained run uses (`lab::packers`, `lab::shopping`), so the only
//! thing that differs between this and a trained run is which road verb gets
//! pressed - which is exactly the comparison worth having.

use gearmaster_console::{Console, Door, Verb};
use gearmaster_trades::env::Step as RoadStep;
use gearmaster_trades::quest::{Progress, Quest};

/// The one thing a written road policy has to remember.
///
/// **Which rung it last packed on.** Without it the priority below is a loop:
/// a run that fights before it packs loses rung one, a Grinder cannot slide
/// below rung one, and the same fight is offered again for ever - which is what
/// the first version did, three hundred and twenty times an episode, and it
/// reads exactly like a policy that has decided fighting is good.
///
/// `hands::pack` leaves in the tray whatever it would not seat, so "the tray is
/// empty" is not the question either. The question is whether this rung has had
/// its packing yet.
#[derive(Default)]
pub struct Written {
    packed_at: Option<usize>,
}

/// Which of the offered steps to take, knowing the plan.
///
/// Priority, and the order is the argument:
///
/// 1. **A door the plan names.** The only irreversible decision in a chain is
///    which choice is taken at its stations, so it comes first and it is read
///    off the stop's own `by` - a set, so the Slagworks route is as good as the
///    astronomer's.
/// 2. **Any other door**, first open choice. A door left standing blocks the
///    ones under it (`road_stack`), so the road does not move until it is
///    answered.
/// 3. **A town door the plan names**, while the plan still wants something -
///    which is the bar, for a chain that starts with a word.
/// 4. **The board, once a rung, before the fight.** See `Written`.
/// 5. Levers, fountains, dungeons, and then the fight. A run that never fights
///    never reaches a rung, and every window in every chain is written in rungs.
impl Written {
    pub fn choose(&mut self, q: &Quest, p: &Progress, c: &Console, ms: &[RoadStep]) -> usize {
        choose(self, q, p, c, ms)
    }
}

fn choose(w: &mut Written, q: &Quest, p: &Progress, c: &Console, ms: &[RoadStep]) -> usize {
    let v = c.view();
    let index = |f: &dyn Fn(&Verb) -> bool| {
        ms.iter().position(|s| matches!(s, RoadStep::Press(x) if f(x)))
    };

    // 1 and 2: whatever is asking.
    if let Some(qn) = &v.question {
        let wanted: Vec<&String> = q
            .stops
            .iter()
            .enumerate()
            .filter(|(i, _)| !p.has(*i))
            .flat_map(|(_, s)| s.by.iter())
            .collect();
        let pick = qn
            .choices
            .iter()
            .filter(|ch| ch.open)
            .find(|ch| wanted.iter().any(|w| **w == ch.label))
            .or_else(|| qn.choices.iter().find(|ch| ch.open))
            .map(|ch| ch.index);
        if let Some(choice) = pick {
            if let Some(at) = index(&|x| matches!(x, Verb::Answer { choice: k } if *k == choice)) {
                return at;
            }
        }
    }

    // 3: a gate, and the door of it the plan wants.
    if v.town.is_some() {
        let wanted: Vec<Door> = q
            .stops
            .iter()
            .enumerate()
            .filter(|(i, _)| !p.has(*i))
            .flat_map(|(_, s)| s.doors.iter().copied())
            .collect();
        for d in wanted {
            if let Some(at) = index(&|x| matches!(x, Verb::Town { door } if *door == d)) {
                return at;
            }
        }
        // Nothing the plan wants here. Walk on rather than spend the visit -
        // a town is one action and the chain may want the next one.
        if let Some(at) = index(&|x| matches!(x, Verb::WalkOn)) {
            return at;
        }
    }

    // 4: the board, before the fight and once a rung. A run that fights on
    // what it was dealt loses rung one for ever.
    if w.packed_at != Some(v.rung_shown) {
        if let Some(at) = ms.iter().position(|s| matches!(s, RoadStep::Pack)) {
            w.packed_at = Some(v.rung_shown);
            return at;
        }
    }

    // 5: everything else, in the order that keeps a run moving.
    for f in [
        &|x: &Verb| matches!(x, Verb::ThrowPoints { .. }) as bool as _,
        &|x: &Verb| matches!(x, Verb::Drink) as bool as _,
        &|x: &Verb| matches!(x, Verb::FightParty) as _,
        &|x: &Verb| matches!(x, Verb::Fight) as _,
    ] as [&dyn Fn(&Verb) -> bool; 4]
    {
        if let Some(at) = index(f) {
            return at;
        }
    }
    // A packing, if one is offered and the board has room to change; otherwise
    // whatever is first, which at this point is a run with nothing to do.
    ms.iter().position(|s| matches!(s, RoadStep::Pack)).unwrap_or(0)
}
