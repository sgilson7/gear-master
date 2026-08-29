//! Does the pilot sell the rumours it barters for?
use gearmaster_engine::piece::CATALOG;
use gearmaster_engine::rating::resale_price;
use gearmaster_engine::rumour::RUMOURS;

fn main() {
    println!("{:<34} {:>6} {:>8}  {}", "rumour", "price", "resale", "cells");
    for r in RUMOURS {
        match CATALOG.iter().find(|d| d.name == r.name) {
            Some(d) => println!(
                "{:<34} {:>6} {:>8}  {}",
                r.name,
                d.price,
                resale_price(d),
                d.cells.len()
            ),
            None => println!("{:<34} not a component at all", r.name),
        }
    }
    // What the cheapest thing in a typical tray costs, for comparison.
    let mut prices: Vec<i32> = CATALOG.iter().map(|d| d.price).collect();
    prices.sort_unstable();
    println!(
        "\ncatalogue prices: min {}, 10th percentile {}, median {}",
        prices[0],
        prices[prices.len() / 10],
        prices[prices.len() / 2]
    );
}
