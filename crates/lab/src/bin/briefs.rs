//! What each theme asks for, and how alike the briefs are.
//!
//!     cargo run --release -p gearmaster-lab --bin briefs
//!
//! Q8's premise check, and it has to run before the training does: if two
//! themes have the same brief the conditioning cannot separate them, and if a
//! held-out theme is unlike everything trained then generalisation was never
//! on the table. Neither is a result about learning - both are results about
//! thirteen numbers - and both are cheap to find out first.

use gearmaster_lab::themes;

fn main() {
    println!("# The briefs\n");
    println!(
        "`cargo run --release -p gearmaster-lab --bin briefs`. Thirteen numbers a \
         theme: five grids,\nthen eight pools scaled so the largest is one.\n"
    );
    let pools = gearmaster_console::view::POOLS;
    print!("| theme | helm | chest | glove | greav | weap |");
    for p in pools {
        print!(" {} |", p);
    }
    println!();
    println!("|---|{}|", "---:|".repeat(13));
    for t in themes::ALL {
        let b = themes::brief(t);
        print!(
            "| {}{} |",
            themes::name(t),
            if themes::HELD_OUT.contains(&t) { " *(held out)*" } else { "" }
        );
        for x in b.0 {
            if x == 0.0 {
                print!(" · |");
            } else {
                print!(" {:.2} |", x);
            }
        }
        println!();
    }

    println!("\n## How alike, as the cosine between them\n");
    print!("| |");
    for t in themes::ALL {
        print!(" {} |", &themes::name(t)[..3]);
    }
    println!();
    println!("|---|{}|", "---:|".repeat(10));
    for a in themes::ALL {
        print!("| **{}** |", themes::name(a));
        for b in themes::ALL {
            let l = themes::brief(a).likeness(&themes::brief(b));
            print!(" {:.2} |", l);
        }
        println!();
    }

    println!("\n## What a held-out theme is nearest to\n");
    for h in themes::HELD_OUT {
        let mut near: Vec<(f32, String)> = themes::trained()
            .into_iter()
            .map(|t| (themes::brief(h).likeness(&themes::brief(t)), themes::name(t)))
            .collect();
        near.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        println!(
            "  - **{}** — {}",
            themes::name(h),
            near.iter().take(3).map(|(l, n)| format!("{} {:.2}", n, l)).collect::<Vec<_>>().join(", ")
        );
    }
}
