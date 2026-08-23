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
            ["shop"] => {
                println!("\nGold: {}   |   next: {}", run.gold, run.monster().name);
                for (i, &def) in run.shop.stock.iter().enumerate() {
                    let d = &gearmaster_engine::piece::CATALOG[def];
                    let afford = if run.gold >= d.price { " " } else { "!" };
                    println!(
                        "{} {}. {:<18} {:<8} {:<10} {:>3}g   {}",
                        afford, i, d.name, d.slot.name().to_lowercase(), d.kind.name_in(d.slot), d.price,
                        d.base.summary()
                    );
                    for t in d.triggers {
                        println!("       {}", t.describe());
                    }
                    if let Some(e) = d.effect {
                        println!("       {}", e.describe());
                    }
                }
            }
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
    println!("  slots: helmet chest gloves greaves weapon");
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
            if let Some(b) = def.adjacency {
                println!("        on assembly: {}", b.label);
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
            def.slot.name().to_lowercase(),
            def.kind.name_in(def.slot),
            shape.width(),
            shape.height(),
            def.base.summary(),
            def.adjacency
                .map(|b| format!("   [on assembly: {}]", b.label))
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
