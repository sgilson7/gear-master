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
