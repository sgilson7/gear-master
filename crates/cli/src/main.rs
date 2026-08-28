//! Headless driver. Plays the whole game — equip, assemble, fight — with no
//! window, so the prototype can be exercised in a script or in CI.
//!
//!   printf 'preset\nstats\nfight\n' | cargo run -q -p gearmaster-cli

use std::io::{self, BufRead, Write};

use gearmaster_engine::piece::{PieceId, SlotKind};
use gearmaster_engine::combat::LADDER;
use gearmaster_engine::run::Run;
use gearmaster_engine::slot::{SLOT_H, SLOT_W};

fn main() {
    let mut run = Run::new();
    println!("Gear Master — type `help` for commands.");

    let stdin = io::stdin();
    let mut line = String::new();
    loop {
        print!("> ");
        io::stdout().flush().ok();
        line.clear();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            [] => continue,
            ["quit"] | ["exit"] => break,
            ["help"] => help(),
            ["inv"] => show_inventory(&run),
            ["show"] => show_all_slots(&run),
            ["show", slot] => match parse_slot(slot) {
                Some(k) => show_slot(&run, k),
                None => println!("error: unknown slot '{}'", slot),
            },
            ["stats"] => show_stats(&run),
            ["shop"] => show_shop(&run),
            ["buy", n] => match n.parse::<usize>() {
                Ok(i) => match run.buy(i) {
                    Ok(id) => println!(
                        "Bought {} for {}g. {} gold left.",
                        run.registry.def(id).name,
                        run.registry.def(id).price,
                        run.gold
                    ),
                    Err(e) => println!("error: {}", e),
                },
                Err(_) => println!("usage: buy <shop index>"),
            },
            ["sell", rest @ ..] if !rest.is_empty() => match find(&run, &rest.join(" ")) {
                Some(id) => {
                    let name = run.registry.def(id).name;
                    match run.sell(id) {
                        Ok(g) => println!("Sold {} for {}g. {} gold.", name, g, run.gold),
                        Err(e) => println!("error: {}", e),
                    }
                }
                None => println!("error: no piece matching '{}'", rest.join(" ")),
            },
            ["ladder"] => {
                println!();
                for (i, m) in LADDER.iter().enumerate() {
                    let here = if i == run.rung { "->" } else { "  " };
                    let (stats, items) = m.outfit();
                    println!(
                        "{} {:<16} {:>4} hp  {:>2} str  {}.{:02}x  {:>2} regen  mind {:>2}%  curse {:>2}%  {:>3}g",
                        here, m.name, stats.health, stats.strength,
                        stats.power / 100, stats.power % 100,
                        stats.regen, stats.mind_resist, stats.curse_resist, m.bounty
                    );
                    for a in m.attacks {
                        println!(
                            "     (innate) {:<12} every {:.1}s  {}{}{}",
                            a.name,
                            a.cooldown_ms as f32 / 1000.0,
                            if a.damage > 0 { format!("{} dmg ", a.damage) } else { String::new() },
                            if a.mind > 0 { format!("{} mind ", a.mind) } else { String::new() },
                            a.curse.map(|c| format!("curse of {}", c.name())).unwrap_or_default()
                        );
                    }
                    for it in &items {
                        println!(
                            "     {:<34} every {:.2}s  {}",
                            it.name, it.cooldown_ms as f32 / 1000.0, it.stats.summary()
                        );
                    }
                }
            }
            ["preset"] => {
                run.apply_preset();
                println!("Applied the full preset loadout.");
                show_stats(&run);
            }
            ["sandbox"] => {
                // Every component, for trying combinations out without playing
                // a run up to them.
                run = Run::with_all_pieces();
                println!("Sandbox: you now own all {} components.", run.owned.len());
            }
            ["clear"] => {
                run.clear_all();
                println!("Cleared every slot.");
            }
            // Component names contain spaces, so the trailing three arguments
            // are peeled off the end and everything before them is the name.
            ["equip", rest @ ..] if rest.len() >= 4 => {
                let (name_parts, tail) = rest.split_at(rest.len() - 3);
                let name = name_parts.join(" ");
                let (Some(kind), Ok(x), Ok(y)) =
                    (parse_slot(tail[0]), tail[1].parse(), tail[2].parse())
                else {
                    println!("usage: equip <name> <slot> <x> <y>");
                    continue;
                };
                match find(&run, &name) {
                    Some(id) => match run.equip(id, kind, x, y) {
                        Ok(()) => {
                            let a = run.report(kind);
                            println!(
                                "Equipped {} -> {} ({}, {}). {} [{}]",
                                run.registry.def(id).name,
                                kind.name(),
                                x,
                                y,
                                a.summary(),
                                a.stats.summary()
                            );
                        }
                        Err(e) => println!("error: {}", e),
                    },
                    None => println!("error: no piece matching '{}'", name),
                }
            }
            ["unequip", rest @ ..] if !rest.is_empty() => match find(&run, &rest.join(" ")) {
                Some(id) => match run.unequip(id) {
                    Ok(()) => println!("Removed {}.", run.registry.def(id).name),
                    Err(e) => println!("error: {}", e),
                },
                None => println!("error: no piece matching '{}'", rest.join(" ")),
            },
            ["rotate", rest @ ..] if !rest.is_empty() => match find(&run, &rest.join(" ")) {
                Some(id) => match run.rotate(id) {
                    Ok(()) => println!(
                        "Rotated {} (now {} quarter turns).",
                        run.registry.def(id).name,
                        run.registry.rotation(id)
                    ),
                    Err(e) => println!("error: {}", e),
                },
                None => println!("error: no piece matching '{}'", rest.join(" ")),
            },
            // ----------------------------------------------- the road
            //
            // Everything standing on a rung besides the fight, and the way
            // through it. Without these the headless driver could equip and
            // fight and nothing else, so a scripted run walked past every
            // event, town and fountain in the game without saying so - which
            // makes "two replays produce identical logs" a claim about a road
            // nobody was on.
            ["road"] => show_road(&run),
            ["map"] => {
                for l in gearmaster_engine::route::ascii(&run) {
                    println!("{}", l);
                }
            }
            ["answer", n] => match n.parse::<usize>() {
                Ok(i) => answer(&mut run, i, None),
                Err(_) => println!("error: answer <n>"),
            },
            // One door in the game asks for a number, and a number cannot be
            // guessed on the player's behalf: a default bid is a bid nobody
            // made.
            ["answer", n, fig] => match (n.parse::<usize>(), fig.parse::<i32>()) {
                (Ok(i), Ok(f)) => answer(&mut run, i, Some(f)),
                _ => println!("error: answer <n> <figure>"),
            },
            ["town"] => show_town(&run),
            ["town", "on"] => {
                let paid = run.skip_town();
                println!("Walked on. +{}g.", paid);
                print_receipt(&mut run);
            }
            ["town", door] => match parse_door(door) {
                Some(a) => {
                    run.visit_town(a);
                    print_receipt(&mut run);
                }
                None => println!("error: chapel, pub, factory, shop, county"),
            },
            // --- THE HUNDRED ---------------------------------------------
            //
            // `go` is the way down from a town, `walk` is a move and `out`
            // walks back up. Deliberately not `leave`: that verb already means
            // "out of a dungeon" and a run can be standing in a county and
            // have a dungeon underneath it, so one word for two doors would
            // pick the wrong one exactly when it mattered.
            ["go"] => {
                let door = gearmaster_engine::town::Action::County;
                if run.county_at.is_some() {
                    println!("You are already down there. `walk n|s|e|w`, or `out`.");
                } else if run.pending_town().is_none() {
                    println!("The way down is in a town, and you are not at one.");
                } else {
                    run.visit_town(door);
                    print_receipt(&mut run);
                    show_county(&run);
                }
            }
            ["walk", dir] => match gearmaster_engine::county::Step::parse(dir) {
                None => println!("error: walk n|s|e|w"),
                Some(step) => {
                    if run.county_at.is_none() {
                        println!("You are not in THE HUNDRED.");
                    } else {
                        run.county_walk(step);
                        print_receipt(&mut run);
                        show_county(&run);
                    }
                }
            },
            ["out"] => {
                if run.leave_county() {
                    print_receipt(&mut run);
                    show_road(&run);
                } else {
                    println!("You are not in THE HUNDRED.");
                }
            }
            ["throw", n] => match n.parse::<usize>() {
                Ok(i) if run.throw_points(i) => {
                    print_receipt(&mut run);
                    show_road(&run);
                }
                Ok(_) if !run.at_points => println!("You are not at the points."),
                Ok(i) => println!("error: there is no road {} here", i),
                Err(_) => println!("error: throw <n>"),
            },
            // The pedestal takes an item as its argument, which no other verb
            // does, so it is `pedestal <n>` against the inventory rather than
            // a door name. Added because `feed_pedestal` had no caller in
            // either driver: six destinations existed and a scripted run could
            // not reach one of them.
            ["pedestal", n] => match n.parse::<usize>() {
                Ok(i) => match run.inventory().get(i.saturating_sub(1)).copied() {
                    None => println!("error: no item {} in the inventory", i),
                    Some(id) => {
                        let name = run.registry.def(id).name;
                        match run.feed_pedestal(id) {
                            Some(d) => {
                                println!("The socket takes it. It goes to {}.", d.name);
                                print_receipt(&mut run);
                                show_road(&run);
                            }
                            None => println!(
                                "The socket does not want {}. It wants an Orb of Travel you have not spent.",
                                name
                            ),
                        }
                    }
                },
                Err(_) => println!("error: pedestal <n>"),
            },
            ["leave"] => {
                if run.leave_dungeon() {
                    print_receipt(&mut run);
                    show_road(&run);
                } else if run.dungeon.is_none() {
                    println!("You are not in a dungeon.");
                } else {
                    println!("Not from here. A landing or the points, and never mid-fight.");
                }
            }
            ["drink"] => {
                if run.at_fountain() {
                    let c = run.drink();
                    println!("The fountain names you {}: {}", c.name, c.blurb);
                } else if run.at_doubling_fountain() {
                    match run.doubling_offer().first().copied() {
                        Some(c) => {
                            run.double_class(c);
                            println!("Doubled: {}", c.name);
                        }
                        None => println!("The deep fountain has nothing of yours to double."),
                    }
                } else {
                    println!("No fountain here.");
                }
            }
            ["fight"] => {
                let items = run.combat_items();
                let log = run.fight_next().clone();
                println!(
                    "\n{} - {} hp, {} str, {}.{:02}x power, {} regen/s",
                    log.player.name,
                    log.player.max_health,
                    log.player.strength,
                    log.player.power / 100,
                    log.player.power % 100,
                    log.player.regen
                );
                for it in &items {
                    println!(
                        "    {:<34} every {:.2}s   {}",
                        it.name,
                        it.cooldown_ms as f32 / 1000.0,
                        it.stats.summary()
                    );
                }
                println!(
                    "vs {} - {} hp",
                    log.enemy().name, log.enemy().max_health
                );
                println!("{}", "-".repeat(64));
                for entry in &log.entries {
                    println!("{}", log.describe(entry));
                }
                println!("{}", "-".repeat(64));
                println!(
                    "{} after {:.1}s",
                    log.outcome.label(),
                    log.duration_ms as f32 / 1000.0
                );
                match run.settle() {
                    Some(g) => println!(
                        "+{} gold (now {}). Next up: {}\n",
                        g,
                        run.gold,
                        run.monster().name
                    ),
                    None => println!("No reward. Still facing {}.\n", run.monster().name),
                }
                run.back_to_loadout();
            }
            ["items"] => {
                let items = run.combat_items();
                println!("\n{} assembled item(s) will act in combat:", items.len());
                for it in &items {
                    println!(
                        "  {}",
                        it.full_name
                    );
                    println!(
                        "      {:<10} {:<18} every {:.2}s  {}",
                        format!("{:?}", it.slot).to_lowercase(),
                        format!("[{}]", it.core),
                        it.cooldown_ms as f32 / 1000.0,
                        it.stats.summary()
                    );
                    if it.adjacent_assembled_same_slot > 0 {
                        println!(
                            "      touching {} other assembled item(s) in its slot",
                            it.adjacent_assembled_same_slot
                        );
                    }
                    for t in &it.triggers {
                        println!("      {}", t.describe());
                    }
                }
            }
            // --- the twelve the window had and this driver did not ------
            //
            // A transcript is only a proof of playability if a person can type
            // it, so every verb `gearmaster-console` offers has a spelling
            // here. Twelve of these were reachable in the window and not here
            // (`console/tests/parity.rs`), and four - `clear <slot>`, `crush`,
            // `grow` and `perambulate` - were reachable in neither, which is
            // how a chain shipped with no way to walk it.
            ["reroll"] => match run.reroll() {
                Ok(()) => {
                    println!("New shelves. {} gold left.", run.gold);
                    show_shop(&run);
                }
                Err(e) => println!("error: {}", e),
            },
            ["pin", n] => match n.parse::<usize>() {
                Ok(i) if i < run.shop.stock.len() => {
                    let held = run.shop.toggle_lock(i);
                    println!("{} shelf {}.", if held { "Holding" } else { "Let go of" }, i);
                }
                _ => println!("usage: pin <shelf>"),
            },
            ["barter", n, rest @ ..] if !rest.is_empty() => {
                match (n.parse::<usize>(), find(&run, &rest.join(" "))) {
                    (Ok(i), Some(id)) => match run.barter(i, id) {
                        Ok(got) => println!("Traded for {}.", run.registry.def(got).name),
                        Err(e) => println!("error: {}", e),
                    },
                    (_, None) => println!("error: no piece matching '{}'", rest.join(" ")),
                    _ => println!("usage: barter <shelf> <name>"),
                }
            }
            ["undo"] => match run.undo() {
                Some(what) => println!("Took back {}.", what),
                None => println!("Nothing to take back."),
            },
            ["lock", rest @ ..] if !rest.is_empty() => match find(&run, &rest.join(" ")) {
                Some(id) => {
                    let now = run.toggle_lock_item(id);
                    println!("{}.", if now { "Locked" } else { "Unlocked" });
                }
                None => println!("error: no piece matching '{}'", rest.join(" ")),
            },
            // A locked item moves as one shape, which is three more verbs and
            // not one: lift it, turn it, put it down.
            ["lift", rest @ ..] if !rest.is_empty() => match find(&run, &rest.join(" ")) {
                Some(id) => match run.unequip_locked(id) {
                    Ok(()) => println!("Lifted the item."),
                    Err(e) => println!("error: {}", e),
                },
                None => println!("error: no piece matching '{}'", rest.join(" ")),
            },
            ["turn", rest @ ..] if !rest.is_empty() => match find(&run, &rest.join(" ")) {
                Some(id) => match run.rotate_locked(id) {
                    Ok(()) => println!("Turned the item."),
                    Err(e) => println!("error: {}", e),
                },
                None => println!("error: no piece matching '{}'", rest.join(" ")),
            },
            // Same peeling as `equip`: a component name has spaces in it.
            ["drop", rest @ ..] if rest.len() >= 4 => {
                let (name_parts, tail) = rest.split_at(rest.len() - 3);
                let name = name_parts.join(" ");
                let (Some(k), Ok(ax), Ok(ay)) =
                    (parse_slot(tail[0]), tail[1].parse::<u8>(), tail[2].parse::<u8>())
                else {
                    println!("usage: drop <name> <slot> <x> <y>");
                    continue;
                };
                match find(&run, &name) {
                    Some(id) => match run.equip_locked_at(id, k, ax, ay) {
                        Ok(()) => show_slot(&run, k),
                        Err(e) => println!("error: {}", e),
                    },
                    None => println!("error: no piece matching '{}'", name),
                }
            }
            ["clear", slot] => match parse_slot(slot) {
                Some(k) => match run.clear_slot(k) {
                    Ok(()) => println!("Emptied the {}.", k.name().to_lowercase()),
                    Err(e) => println!("error: {}", e),
                },
                None => println!("error: unknown slot '{}'", slot),
            },
            ["grow", slot] => match parse_slot(slot) {
                Some(k) => {
                    if run.grow_slot(k) {
                        print_receipt(&mut run);
                        show_slot(&run, k);
                    } else {
                        println!("No row owed.");
                    }
                }
                None => println!("error: unknown slot '{}'", slot),
            },
            ["crush", rest @ ..] if !rest.is_empty() => match find(&run, &rest.join(" ")) {
                Some(id) => {
                    let name = run.registry.def(id).name;
                    match run.crush(id) {
                        Some(_) => {
                            println!("Crushed {}.", name);
                            print_receipt(&mut run);
                            show_road(&run);
                        }
                        None => println!("{} does not crush, or not here.", name),
                    }
                }
                None => println!("error: no piece matching '{}'", rest.join(" ")),
            },
            // The tenth trip: a route rather than a destination. Nothing in
            // either driver could walk one before this, which is why THE
            // PARISH had never been reached outside a test.
            ["perambulate", x, y] => match (x.parse::<u8>(), y.parse::<u8>()) {
                (Ok(mx), Ok(my)) => {
                    if run.walk_the_perambulation((mx, my)) {
                        print_receipt(&mut run);
                        show_county(&run);
                    } else {
                        println!("Not granted, or not from a mouth.");
                    }
                }
                _ => println!("usage: perambulate <x> <y>"),
            },
            ["mouths"] => {
                for (town, at) in gearmaster_engine::county::MOUTHS {
                    println!("  {:<14} {}  ({}, {})", town, gearmaster_engine::county::reference(at), at.0, at.1);
                }
            }
            ["drink", n] => match n.parse::<usize>() {
                Ok(i) => {
                    let offer = run.fountain_offer();
                    match offer.get(i).copied() {
                        Some(c) => match run.drink_choosing(c) {
                            Some(got) => println!("The fountain names you {}: {}", got.name, got.blurb),
                            None => println!("It will not."),
                        },
                        None => {
                            let dbl = run.doubling_offer();
                            match dbl.get(i).copied() {
                                Some(c) if run.double_class(c) => println!("Doubled: {}", c.name),
                                _ => println!("It is not offering that."),
                            }
                        }
                    }
                }
                Err(_) => println!("usage: drink [n]"),
            },
            ["brawl"] => match run.pending_brawl() {
                None => println!("No brawl stands here."),
                Some(specs) => {
                    let log = run.fight_party(&specs).clone();
                    println!(
                        "{} after {:.1}s",
                        log.outcome.label(),
                        log.duration_ms as f32 / 1000.0
                    );
                    match run.settle() {
                        Some(g) => println!("+{} gold (now {}).", g, run.gold),
                        None => println!("No reward."),
                    }
                    run.back_to_loadout();
                    show_road(&run);
                }
            },
            _ => println!("unknown command; try `help`"),
        }
    }
}

fn help() {
    println!("  show [slot]              draw the slot grids");
    println!("  inv                      list unequipped components");
    println!("  stats                    character totals and per-slot assembly");
    println!("  equip <name> <slot> <x> <y>");
    println!("  unequip <name>           send a component back to the inventory");
    println!("  rotate <name>            quarter turn clockwise");
    println!("  preset | clear           fill or empty every slot");
    println!("  sandbox                  grant every component (testing)");
    println!("  shop                     what is for sale");
    println!("  buy <n> | sell <name>    trade with the shop");
    println!("  ladder                   the monster ladder");
    println!("  items                    list the items that will act in combat");
    println!("  fight                    simulate and print the whole bout");
    println!("  road                     what is standing on this rung");
    println!("  map                      the whole road and the county under it");
    println!("  answer <n> [figure]      take choice n at the event in front of you");
    println!("  town | town on | town <door>   the gate, walking past it, or going in");
    println!("  drink                    take what the fountain is offering");
    println!("  throw <n>                at a set of points, take road n");
    println!("  pedestal <n>             feed inventory item n to the pedestal");
    println!("  leave                    walk out of a dungeon, keeping what you cleared");
    println!("  go                       from a town, down into THE HUNDRED");
    println!("  walk n|s|e|w             one tile of the county");
    println!("  out                      back up out of the county");
    println!("  reroll | pin <n> | barter <n> <name>   the rest of the shop");
    println!("  undo                     take back the last board change");
    println!("  lock <name>              lock an assembled item, or unlock it");
    println!("  lift | turn | drop <name> [slot x y]    move a locked item as one shape");
    println!("  clear <slot> | grow <slot>              one grid");
    println!("  crush <name>             break a relic for what is inside it");
    println!("  drink <n>                take the nth thing a fountain offers");
    println!("  brawl                    walk into a fight an event arranged");
    println!("  mouths                   where the six ways into the county are");
    println!("  perambulate <x> <y>      the tenth trip: a route, from a mouth");
    println!("  slots: helmet chest gloves greaves weapon");
}

/// The receipt from whatever was just resolved, if there is one.
///
/// Read once and dismissed, which is what makes it a receipt rather than a
/// status line: it describes a moment, and the road moves on after it.
fn print_receipt(run: &mut Run) {
    if let Some(lines) = run.take_receipt() {
        println!("  ----");
        for l in lines {
            println!("  {}", l);
        }
        println!("  ----");
    }
}

/// The road stack, drawn the way the interface draws it: what you are in,
/// what is under it, and the rung's own fight as the floor so the queue
/// visibly ends somewhere.
/// Where you are standing in THE HUNDRED, and what is around you.
///
/// The five tiles a move can reach, named. The map itself is F9's; this is
/// the driver being able to say where it is at all, which is what makes a
/// scripted trip a thing that can be diffed.
fn show_county(run: &Run) {
    use gearmaster_engine::county;
    let Some(at) = run.county_at else { return };
    // A question on the tile you are standing on is the only thing you can do
    // next, so it is the only thing worth printing.
    if run.pending_event().is_some() {
        show_question(run);
        return;
    }
    let c = run.county();
    // Read off `describe()` rather than formatted here, for `show_road`'s
    // reason: the banner is a reading of the run now, and two interfaces
    // working one out separately are two interfaces that will disagree about
    // it one day.
    if let Some(i) = run.road_stack().iter().find(|i| i.kind() == "county") {
        println!("\n{}", run.theme.retell(&i.describe()).to_uppercase());
    }
    println!("  {}", run.theme.retell(c.at(at).kind.what()));
    for step in county::Step::ALL {
        match step.from(at) {
            None => println!(
                "  {}  {}",
                step.key(),
                run.theme.retell("the edge of the county")
            ),
            Some(to) => {
                let t = c.at(to);
                let mark = if run.county_is_cleared(to) {
                    "#"
                } else if c.is_sealed(to) && !run.pale_is_open() {
                    "|"
                } else {
                    "."
                };
                println!(
                    "  {}  {} {} {}",
                    step.key(),
                    mark,
                    county::reference(to),
                    run.theme.retell(t.kind.what())
                );
            }
        }
    }
}

fn show_road(run: &Run) {
    let stack = run.road_stack();
    // You always know you are inside one. Read off `describe()` rather than
    // formatted here: the banner's two numbers are a reading of the run now -
    // fights won this entry, and floors walked past because they were already
    // beaten - and two interfaces working them out separately is two
    // interfaces that will one day disagree.
    if let Some(i) = stack.iter().find(|i| i.kind() == "dungeon") {
        println!("\n{}", run.theme.retell(&i.describe()).to_uppercase());
    }
    println!("\nRung {} - {}", run.rung + 1, run.monster().name);
    if stack.is_empty() {
        println!("  nothing on the road. `fight`.");
    }
    for (i, it) in stack.iter().enumerate() {
        let head = if i == 0 { "->" } else { "  " };
        println!("  {} {:<10} {}", head, it.kind(), it.describe());
    }
    if !stack.is_empty() {
        println!("     {:<10} {}", "fight", run.monster().name);
    }
    // At the points, the roads out are the question, printed the way an
    // event's choices are printed because that is what they are.
    if let Some((d, floor)) = run.dungeon.filter(|_| run.at_points) {
        for line in d.floors[floor].fork {
            println!("  {}", run.theme.retell(line));
        }
        for (i, e) in d.floors[floor].exits.iter().enumerate() {
            let walked = run.has_cleared(d.id, e.to);
            println!("    {}. {}{}", i, e.label, if walked { "   (cleared)" } else { "" });
            println!("        {}", e.blurb);
        }
        println!("    `throw <n>`, or `leave`.");
    }
    show_question(run);
}

/// Whatever is asking you something, and what it will take for an answer.
///
/// Split out of `show_road` because `show_county` needs it too, and needed it
/// before this existed: a question set by walking onto a tile was printed by
/// nothing, so a player walked five tiles past it and met it on the road one
/// town later. A screen that can hold a question has to draw it.
fn show_question(run: &Run) {
    if let Some(e) = run.pending_event() {
        println!("\n{}", run.theme.place(e.id, e.title));
        for line in run.theme.scene(e.id, e.prose) {
            println!("  {}", run.theme.retell(line));
        }
        for (i, c) in e.choices.iter().enumerate() {
            let open = run.choice_open(c);
            println!("  {} {}. {}", if open { " " } else { "!" }, i, c.label);
            println!("        {}", c.blurb);
            if !open {
                // The plain statement before an attempt, and the flavour for
                // after one. Both, because they are different sentences.
                println!("        {}", c.requires.describe());
                println!("        {}", c.unmet);
            }
        }
        println!("    `answer <n>`.");
    }
}

fn answer(run: &mut Run, i: usize, figure: Option<i32>) {
    let Some(e) = run.pending_event() else {
        println!("Nothing is asking you anything.");
        return;
    };
    let Some(c) = e.choices.get(i) else {
        println!("error: {} has {} choices", e.title, e.choices.len());
        return;
    };
    let took = match figure {
        Some(f) => run.take_choice_with(c, f),
        None => run.take_choice(c),
    };
    if took.is_none() && run.last_receipt.is_none() {
        println!("{}", c.requires.describe());
        println!("{}", c.unmet);
        return;
    }
    println!("{}", c.label);
    print_receipt(run);
}

fn show_town(run: &Run) {
    let Some(t) = run.pending_town() else {
        println!("No gate here.");
        return;
    };
    println!("\n{}", t.name);
    for line in t.blurb {
        println!("  {}", line);
    }
    println!("  One of these, and then the road:");
    for a in t.actions.iter().copied() {
        println!("    {:<9} {}", format!("{:?}", a).to_lowercase(), a.blurb());
    }
    println!("    on        walk past, and take the bounty again");
}

fn parse_door(s: &str) -> Option<gearmaster_engine::town::Action> {
    use gearmaster_engine::town::Action;
    match s.to_lowercase().as_str() {
        "chapel" => Some(Action::Chapel),
        "pub" => Some(Action::Pub),
        "factory" | "works" => Some(Action::Factory),
        "shop" | "cart" => Some(Action::Shop),
        "pedestal" | "socket" => Some(Action::Pedestal),
        "county" | "steps" | "down" => Some(Action::County),
        _ => None,
    }
}

fn parse_slot(s: &str) -> Option<SlotKind> {
    match s.to_lowercase().as_str() {
        "helmet" | "helm" | "h" => Some(SlotKind::Helmet),
        "chest" | "chestpiece" | "c" => Some(SlotKind::Chest),
        "gloves" | "glove" | "g" => Some(SlotKind::Gloves),
        "greaves" | "greave" | "r" => Some(SlotKind::Greaves),
        "weapon" | "w" => Some(SlotKind::Weapon),
        _ => None,
    }
}

/// Case-insensitive substring match on the component name.
fn find(run: &Run, needle: &str) -> Option<PieceId> {
    let needle = needle.to_lowercase();
    run.owned
        .iter()
        .copied()
        .find(|&id| run.registry.def(id).name.to_lowercase().contains(&needle))
}

fn show_shop(run: &Run) {
    println!("\nGold: {}   |   reroll {}g   |   next: {}", run.gold, run.reroll_cost(), run.monster().name);
    for (i, &def) in run.shop.stock.iter().enumerate() {
        let d = &gearmaster_engine::piece::CATALOG[def];
        // `run.price` and not `d.price`: the second is the catalogue's number
        // and the first is what `buy` will take, and they differ by the markup
        // from the moment THE TOLLBOOTH is answered. This driver used to print
        // the catalogue's.
        let charged = run.price(i).unwrap_or(d.price);
        let afford = if run.gold >= charged { " " } else { "!" };
        let held = if run.shop.is_locked(i) { "*" } else { " " };
        println!(
            "{}{}{}. {:<18} {:<8} {:<10} {:>3}g",
            afford, held, i, d.name, d.kind.slot_label(d.slot).to_lowercase(), d.kind.name_in(d.slot), charged
        );
        // Grouped by when, the same four groups the cards draw. A flat
        // summary prints a rate beside a quantity and says nothing about
        // which is which - "+2 nature, +8 curse res" is two different kinds
        // of promise on one line.
        // An `OnActivate` trigger is the same group as a per-activation stat
        // figure - both are what one activation hands over - so it goes under
        // the same heading, and the heading appears when either is there.
        // Sunderer is the piece that found this: its stats are damage and its
        // curse is a trigger, so the curse printed under a blank label.
        use gearmaster_engine::piece::Trigger;
        use gearmaster_engine::stats::When;
        let mut on_activation: Vec<String> = Vec::new();
        let mut conditional = Vec::new();
        for t in d.triggers {
            match t {
                Trigger::OnActivate(a) => on_activation.push(a.describe()),
                other => conditional.push(other),
            }
        }
        let mut groups = d.base.summary_by_when();
        if !on_activation.is_empty()
            && !groups.iter().any(|(w, _)| *w == When::OnActivation)
        {
            groups.push((When::OnActivation, String::new()));
        }
        for (when, text) in groups {
            if !text.is_empty() {
                println!("       {:<20} {}", when.heading(), text);
            }
            if when == When::OnActivation {
                let mut label = if text.is_empty() { when.heading() } else { "" };
                for line in &on_activation {
                    println!("       {:<20} {}", label, line);
                    label = "";
                }
            }
        }
        for t in conditional {
            println!("       {:<20} {}", "TRIGGERS", t.describe());
        }
        if let Some(e) = d.effect {
            println!("       {}", e.describe());
        }
    }
}

fn show_all_slots(run: &Run) {
    for kind in SlotKind::ALL {
        show_slot(run, kind);
    }
}

fn show_slot(run: &Run, kind: SlotKind) {
    let slot = run.loadout.slot(kind);
    let rep = run.report(kind);
    println!("\n{} - {}  [{}]", kind.name(), kind.recipe_text(), rep.summary());

    // One letter per piece, numbered by which item it belongs to.
    let groups = slot.groups();
    let letter_of = |id| -> char {
        for (gi, g) in groups.iter().enumerate() {
            if let Some(pi) = g.iter().position(|&p| p == id) {
                return char::from(b'A' + (gi as u8 * 4 + pi as u8) % 26);
            }
        }
        '?'
    };
    for y in 0..SLOT_H {
        let row: String = (0..SLOT_W)
            .map(|x| match slot.get(x, y) {
                Some(id) => letter_of(id),
                None => '.',
            })
            .collect();
        println!("  {} {}", y, row);
    }
    println!("    {}", (0..SLOT_W).map(|x| char::from(b'0' + x)).collect::<String>());

    for (i, item) in rep.items.iter().enumerate() {
        println!(
            "  item {}: {:<38} {:<22} [{}]",
            i + 1,
            if item.assembled { item.name.full.clone() } else { format!("(unfinished)") },
            if item.assembled { String::new() } else { item.status.clone() },
            item.stats.summary()
        );
        for &id in &item.pieces {
            let def = run.registry.def(id);
            println!(
                "    {} {:<18} {:<10} {}",
                letter_of(id),
                def.name,
                def.kind.name_in(def.slot),
                def.base.summary()
            );
            if let Some(e) = def.effect {
                println!("        effect: {}", e.describe());
            }
            if let Some(b) = def.assembly_bonus {
                println!("        on assembly: {} ({})", b.label, b.stats.summary());
            }
        }
        for note in &item.notes {
            println!("      -> {}", note);
        }
    }
    if !rep.stats.summary().is_empty() {
        println!("  slot contributes: {}", rep.stats.summary());
    }
}

fn show_inventory(run: &Run) {
    let inv = run.inventory();
    println!("\nInventory ({} components):", inv.len());
    for id in inv {
        let def = run.registry.def(id);
        let shape = run.registry.shape(id);
        println!(
            "  {:<18} {:<8} {:<10} {}x{}  {}{}",
            def.name,
            def.kind.slot_label(def.slot).to_lowercase(),
            def.kind.name_in(def.slot),
            shape.width(),
            shape.height(),
            def.base.summary(),
            def.assembly_bonus
                .map(|b| format!("   [on assembly: {} ({})]", b.label, b.stats.summary()))
                .or_else(|| def.effect.map(|e| format!("   [{}]", e.describe())))
                .unwrap_or_default()
        );
    }
}

fn show_stats(run: &Run) {
    let s = run.player_stats();
    println!("\nCharacter");
    println!("  health   {}", s.health);
    println!("  strength {}", s.strength);
    println!("  regen    {}/turn", s.regen);
    // Power is per weapon now, so it is printed on each item rather than here.
    let items = run.combat_items();
    let dps: i64 = items.iter().map(|i| i.dps_milli(s.strength)).sum();
    println!("  damage   {}.{} per second across every weapon", dps / 1000, (dps % 1000) / 100);
    for it in &items {
        let hit = it.hit_for(s.strength);
        if hit > 0 {
            println!(
                "             {} hits {} every {:.2}s",
                it.name, hit, it.cooldown_ms as f32 / 1000.0
            );
        }
    }
    println!("Gear");
    for r in run.reports() {
        println!("  {:<11} {:<22} {}", r.slot.name(), r.summary(), r.stats.summary());
        for note in r.notes() {
            println!("                {}", note);
        }
    }
}
