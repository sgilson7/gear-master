use gearmaster_engine::piece::{CATALOG, PieceKind, SlotKind};
use gearmaster_engine::rating::piece_rating;
#[test]
#[ignore]
fn show() {
    for n in ["Manaflay","The Split Wisdom","Tithe Collector","Wrathbreaker","Witherroot"] {
        let d = CATALOG.iter().find(|c| c.name == n).unwrap();
        println!("{:<20} {:>4}  {:?}/{:?}", n, piece_rating(d), d.slot, d.kind);
    }
    let best = CATALOG.iter()
        .filter(|c| c.slot == SlotKind::Weapon && c.kind == PieceKind::Accessory
                    && !gearmaster_engine::piece::is_boss_only(c.name))
        .max_by_key(|c| piece_rating(c)).unwrap();
    println!("best ordinary weapon accessory: {} at {}", best.name, piece_rating(best));
}

/// What the Switchyard's six are worth, against the bands they have to sit in.
///
/// An enchantment is priced at `BOND_POINTS` plus its own stat line plus what
/// its bonded triggers are worth, and an orb like any other weapon core. The
/// bands are the shipped pieces': the six enchantments the Unwinding left run
/// from the Lightning Rod at the bottom to Chalked Circle at the top, and the
/// four Orbs of Travel sit in a hand's width of each other.
#[test]
#[ignore]
fn report_the_yards_worth() {
    use gearmaster_engine::piece::{is_event_only, PieceKind, CATALOG};
    use gearmaster_engine::rating::piece_rating;

    println!("\n## The shipped enchantments, by slot\n");
    let mut ships: Vec<&gearmaster_engine::piece::PieceDef> = CATALOG
        .iter()
        .filter(|d| d.kind.is_enchantment() && !is_event_only(d.name))
        .collect();
    ships.sort_by_key(|d| piece_rating(d));
    for d in &ships {
        println!("  {:<22} {:>4}  {:?}  price {}", d.name, piece_rating(d), d.slot, d.price);
    }
    let lo = piece_rating(ships[0]);
    let hi = piece_rating(ships[ships.len() - 1]);
    println!("  band: {lo} to {hi}");

    println!("\n## The yard's four, and where they fall\n");
    for name in ["Ballast Bed", "Points Rodding", "Booking Hall", "Signal Wire"] {
        let d = CATALOG.iter().find(|d| d.name == name).expect("M5");
        let r = piece_rating(d);
        let inside = if r >= lo && r <= hi { "inside" } else { "OUTSIDE" };
        println!("  {:<22} {:>4}  {:?}  price {}  {inside}", d.name, r, d.slot, d.price);
    }

    println!("\n## The four Orbs of Travel\n");
    let mut orbs: Vec<&gearmaster_engine::piece::PieceDef> = CATALOG
        .iter()
        .filter(|d| d.kind == PieceKind::Orb && gearmaster_engine::pedestal::is_orb_of_travel(d.name))
        .filter(|d| !is_event_only(d.name))
        .collect();
    orbs.sort_by_key(|d| piece_rating(d));
    for d in &orbs {
        println!("  {:<22} {:>4}  price {}", d.name, piece_rating(d), d.price);
    }
    let (olo, ohi) = (piece_rating(orbs[0]), piece_rating(orbs[orbs.len() - 1]));
    println!("  band: {olo} to {ohi}");

    println!("\n## The yard's two\n");
    for name in ["Shunter's Orb", "Signalman's Orb"] {
        let d = CATALOG.iter().find(|d| d.name == name).expect("M5");
        let r = piece_rating(d);
        let inside = if r >= olo && r <= ohi { "inside" } else { "OUTSIDE" };
        println!("  {:<22} {:>4}  price {}  {inside}", d.name, r, d.price);
    }
}

/// How often a Derail actually finds something, and how much armour a Ballast
/// actually gets to spend.
///
/// `DERAIL_WINDOW` and `BALLAST_FUNDED` were written as starting points and
/// M8 is where they are measured. Both are "what share of the thing does a
/// build that wanted it actually manage", and both are answerable by fighting
/// rather than by argument: put the action on a hand-built item, fight a real
/// creature at the band the yard stands on, and count.
#[test]
#[ignore]
fn report_what_the_two_conditionals_actually_manage() {
    use gearmaster_engine::combat::{simulate, Event, Side, LADDER};
    use gearmaster_engine::loadout::ItemProfile;
    use gearmaster_engine::piece::{Action, SlotKind, Trigger};
    use gearmaster_engine::stats::Stats;

    let item = |name: &str, slot, cd, triggers: Vec<Trigger>| ItemProfile {
        sigil_seed: 0,
        pieces: Vec::new(),
        name: name.to_string(),
        full_name: name.to_string(),
        core: name.to_string(),
        slot,
        cooldown_ms: cd,
        stats: Stats::ZERO,
        triggers,
        adjacent_assembled_same_slot: 0,
        diagonal_items: Vec::new(),
        open_cells: 0,
        attracts_curses: false,
        steady: false,
        overtakes: false,
        power: 100,
        rating: 0,
        power_bonus: 0,
        casts: Vec::new(),
        adjacent_items: Vec::new(),
        aligned_items: Vec::new(),
    };
    let alive = Stats { health: 40_000, ..Stats::ZERO };

    println!("\n## Derail: activations that found something, by creature\n");
    let mut hits = 0usize;
    let mut fires = 0usize;
    for band in [27usize, 28, 29, 30] {
        let foe = &LADDER[band];
        let wire = item(
            "Wire",
            SlotKind::Gloves,
            2_500,
            vec![Trigger::OnActivate(Action::Derail { window_ms: 1_000, back_ms: 600 })],
        );
        let log = simulate(alive, &[wire], foe);
        let fired = log
            .entries
            .iter()
            .filter(|e| matches!(&e.event, Event::Activate { side: Side::Player, .. }))
            .count();
        let caught =
            log.entries.iter().filter(|e| matches!(&e.event, Event::Derailed { .. })).count();
        println!(
            "  {:<24} {:>3} activations, {:>3} caught  ({:.0}%)  [{} items]",
            foe.name,
            fired,
            caught,
            if fired > 0 { caught as f32 / fired as f32 * 100.0 } else { 0.0 },
            foe.gear_at(gearmaster_engine::combat::Difficulty::Medium).len()
        );
        hits += caught;
        fires += fired;
    }
    println!("  overall: {:.2} of every activation finds something", hits as f32 / fires as f32);

    println!("\n## Ballast: armour actually spent, against what was asked for\n");
    for asked in [10i32, 20, 30] {
        // Income rather than a one-off wall. A one-off is worth nothing here
        // and the first run of this probe measured exactly that: against a
        // creature that is hitting you, a wall granted at the bell is eaten
        // before a five-second chest item comes round, and every reading was
        // zero. What a build that wants Ballast actually has is armour
        // *income*, so that is what is put in front of it.
        for wall in [10i32, 30, 60] {
            let armour = item(
                "Wall",
                SlotKind::Greaves,
                1_500,
                vec![Trigger::OnActivate(Action::GainArmor(wall))],
            );
            let bed = item(
                "Bed",
                SlotKind::Chest,
                5_000,
                vec![Trigger::OnActivate(Action::Ballast(asked))],
            );
            let log = simulate(alive, &[armour, bed], &LADDER[29]);
            let spent: i32 = log
                .entries
                .iter()
                .filter_map(|e| match e.event {
                    Event::Grew { side: Side::Player, paid_armor, .. } => Some(paid_armor),
                    _ => None,
                })
                .sum();
            let asks = log
                .entries
                .iter()
                .filter(|e| matches!(&e.event, Event::Activate { side: Side::Player, item, .. } if item == "Bed"))
                .count() as i32;
            println!(
                "  asked {asked:>3} x{asks:<3} = {:>4} wanted, {spent:>4} spent from a {wall:>3} wall  ({:.2})",
                asked * asks,
                if asks > 0 { spent as f32 / (asked * asks) as f32 } else { 0.0 }
            );
        }
    }
}

/// The yard's six sit in the bands the shipped pieces hold.
///
/// A **price** band, which is what the spec's exemplars are: Chalked Circle at
/// 60 is the dearest ground in the game and the Lightning Rod at 34 the
/// cheapest, and the four Orbs of Travel run 20 to 26. A component outside
/// those is a component that reads as a different kind of thing on a shelf it
/// will never be on - which matters anyway, because the tray shows a price.
#[test]
fn the_yards_six_are_priced_like_the_things_they_are() {
    use gearmaster_engine::piece::{is_event_only, PieceKind, CATALOG};

    let band = |mut v: Vec<i32>| {
        v.sort_unstable();
        (v[0], v[v.len() - 1])
    };
    let (lo, hi) = band(
        CATALOG
            .iter()
            .filter(|d| d.kind.is_enchantment() && !is_event_only(d.name))
            .map(|d| d.price)
            .collect(),
    );
    for name in ["Ballast Bed", "Points Rodding", "Booking Hall", "Signal Wire"] {
        let d = CATALOG.iter().find(|d| d.name == name).expect("M5");
        assert!(
            d.price >= lo && d.price <= hi,
            "{name} is {} gold against a shipped band of {lo} to {hi}",
            d.price
        );
    }

    let (olo, ohi) = band(
        CATALOG
            .iter()
            .filter(|d| d.kind == PieceKind::Orb)
            .filter(|d| gearmaster_engine::pedestal::is_orb_of_travel(d.name))
            .filter(|d| !is_event_only(d.name))
            .map(|d| d.price)
            .collect(),
    );
    for name in ["Shunter's Orb", "Signalman's Orb"] {
        let d = CATALOG.iter().find(|d| d.name == name).expect("M5");
        assert!(
            d.price >= olo && d.price <= ohi,
            "{name} is {} gold against a shipped band of {olo} to {ohi}",
            d.price
        );
    }
}

/// Measuring the two conditionals moved nothing that was already in the game.
///
/// `SHUNT_PS`, `BALLAST_FUNDED`, `DERAIL_WINDOW` and `ACCRUED_ASSUMED` price
/// four verbs that six components speak and nothing else does. So a weight
/// moving at M8 can only move those six - and every one of them is event-only,
/// which is what keeps `stepped_component` from re-dressing a creature.
/// `catalog_shape::no_creature_changed_what_it_wears` is the measurement; this
/// is the reason it is allowed to still be green.
#[test]
fn only_the_yards_own_six_speak_the_verbs_the_new_weights_price() {
    use gearmaster_engine::piece::{is_event_only, walk_actions, Action, CATALOG};

    let mut speakers: Vec<&str> = Vec::new();
    for d in CATALOG {
        for t in d.triggers {
            walk_actions(t, &mut |a| {
                if matches!(
                    a,
                    Action::Shunt { .. }
                        | Action::Ballast(_)
                        | Action::Derail { .. }
                        | Action::Accrue { .. }
                ) && !speakers.contains(&d.name)
                {
                    speakers.push(d.name);
                }
            });
        }
    }
    assert_eq!(speakers.len(), 6, "{speakers:?}");
    for n in &speakers {
        assert!(is_event_only(n), "{n} speaks a yard verb and can reach a creature");
    }
}
