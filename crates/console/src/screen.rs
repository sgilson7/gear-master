//! The view, as text.
//!
//! Not a second reading of the run - a rendering of the first one. Everything
//! here comes off the `View`, so the text a person reads and the fields an
//! agent reads cannot drift apart.

use crate::view::*;
use gearmaster_engine::slot::SLOT_W;

pub fn draw(v: &View) -> Vec<String> {
    let mut out = Vec::new();
    let lives = match v.lives_left {
        Some(n) => format!("   lives {}", n),
        None => String::new(),
    };
    out.push(format!(
        "Rung {} - {}   |   {}g   {}W/{}L{}",
        v.rung_shown, v.coming.name, v.gold, v.wins, v.losses, lives
    ));
    if !v.classes.is_empty() {
        out.push(format!("  classes: {}", v.classes.join(", ")));
    }

    for g in &v.grids {
        let filled = g.cells.iter().filter(|c| c.piece.is_some()).count();
        out.push(format!(
            "  {:<8} {:>2}/{:<3} cells   {}",
            crate::verb::slot_key(g.slot),
            filled,
            g.cells.len(),
            g.summary
        ));
        for y in 0..g.rows {
            let row: String = (0..SLOT_W)
                .map(|x| match g.cells[y as usize * SLOT_W as usize + x as usize].item {
                    Some(i) => char::from(b'A' + (i as u8 % 26)),
                    None => {
                        if g.cells[y as usize * SLOT_W as usize + x as usize].piece.is_some() {
                            'o'
                        } else {
                            '.'
                        }
                    }
                })
                .collect();
            out.push(format!("    {} {}", y, row));
        }
    }

    out.push(format!(
        "  you: {} hp  {} str  {}.{:02}x   figures: flow {} phys {} magic {} armour {} fastest {} hedge {}",
        v.stats.health,
        v.stats.strength,
        v.stats.power / 100,
        v.stats.power % 100,
        v.figures.flow,
        v.figures.physical_dps,
        v.figures.magic_dps,
        v.figures.armour_ps,
        v.figures.fastest_ms.map(|m| m.to_string()).unwrap_or_else(|| "-".into()),
        v.figures.curse_resist
    ));

    out.push(format!("  tray {}/{}:", v.tray.len(), v.tray_cap));
    for p in &v.tray {
        out.push(format!(
            "    #{:<4} {:<24} {:<8} {:<10} {}x{}",
            p.id.map(|i| i.0).unwrap_or(0),
            p.name,
            crate::verb::slot_key(p.slot),
            p.role,
            p.width,
            p.height,
        ));
        // Grouped by when it happens, the way the card groups it. A flat line
        // says "+2 nature, +8 curse res" and says nothing about which of the
        // two is a rate.
        for (when, figures) in &p.when {
            out.push(format!("           {:<14} {}", when, figures));
        }
    }

    out.push(format!("  shop (reroll {}g):", v.reroll_cost));
    for s in &v.shop {
        out.push(format!(
            "    {}{} {:<24} {:<8} {:>3}g{}",
            s.index,
            if s.pinned { "*" } else { "." },
            s.piece.name,
            crate::verb::slot_key(s.piece.slot),
            s.price.unwrap_or(s.piece.price),
            if s.affordable { "" } else { "   (too dear)" }
        ));
    }

    if !v.road.is_empty() {
        out.push("  road:".into());
        for (i, r) in v.road.iter().enumerate() {
            out.push(format!(
                "   {} {:<9} {}",
                if i == 0 { "->" } else { "  " },
                r.kind,
                r.describe
            ));
        }
    }

    if let Some(t) = &v.town {
        out.push(format!("  {} - one door, then the road:", t.name));
        for (d, blurb) in &t.doors {
            out.push(format!("    town {:<12} {}", crate::verb::door_key(*d), first_sentence(blurb)));
        }
        out.push("    town on         walk past, and take the bounty again".into());
    }

    if let Some(f) = &v.fountain {
        out.push(if f.doubling { "  a deep fountain:".into() } else { "  a fountain:".into() });
        for (i, (name, blurb)) in f.offer.iter().enumerate() {
            out.push(format!("    {}. {:<18} {}", i, name, first_sentence(blurb)));
        }
    }

    if let Some(p) = &v.points {
        for l in &p.fork {
            out.push(format!("  {}", l));
        }
        for (i, label, blurb, cleared) in &p.exits {
            out.push(format!(
                "    {}. {}{}",
                i,
                label,
                if *cleared { "   (cleared)" } else { "" }
            ));
            out.push(format!("        {}", blurb));
        }
    }

    if let Some(c) = &v.county {
        out.push(format!(
            "  THE HUNDRED {} - {}   {} moves left, {} trips left, clock {}",
            c.reference, c.here, c.moves_left, c.trips_left, c.clock
        ));
        for (key, n) in &c.around {
            match n {
                None => out.push(format!("    {}  the edge of the county", key)),
                Some(n) => out.push(format!(
                    "    {}  {} {} {}{}",
                    key,
                    if n.cleared {
                        "#"
                    } else if n.sealed {
                        "|"
                    } else {
                        "."
                    },
                    n.reference,
                    n.what,
                    n.threshold.as_ref().map(|t| format!("  [{}]", t)).unwrap_or_default()
                )),
            }
        }
    }

    if let Some(q) = &v.question {
        out.push(format!("  {}", q.title));
        for l in &q.scene {
            out.push(format!("    {}", l));
        }
        for c in &q.choices {
            out.push(format!(
                "   {} {}. {}",
                if c.open { " " } else { "!" },
                c.index,
                c.label
            ));
            out.push(format!("        {}", c.blurb));
            if !c.open {
                out.push(format!("        {}", c.requires));
            }
            if let Some((lo, hi)) = c.figure {
                out.push(format!("        a number between {} and {}", lo, hi));
            }
        }
    }

    if !v.receipt.is_empty() {
        out.push("  ----".into());
        for l in &v.receipt {
            out.push(format!("  {}", l));
        }
        out.push("  ----".into());
    }

    if let Some(f) = &v.last_fight {
        out.push(format!(
            "  last: {} against {} after {:.1}s{}",
            f.outcome,
            f.against,
            f.duration_ms as f32 / 1000.0,
            if f.won && !f.board_decided { "   (the clock decided it)" } else { "" }
        ));
    }

    out.push(format!(
        "  coming: {} - {} hp, {} str, {} items",
        v.coming.name,
        v.coming.stats.health,
        v.coming.stats.strength,
        v.coming.brings.len()
    ));

    out
}

fn first_sentence(s: &str) -> String {
    match s.find(". ") {
        Some(i) => s[..=i].to_string(),
        None => s.to_string(),
    }
}
