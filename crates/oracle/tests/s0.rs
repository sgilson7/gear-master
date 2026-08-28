//! The surrogate reads what the engine classifies, and all of it.
//!
//! `s0.rs` keeps a list of `Stats` fields and asks the engine which group each
//! belongs to. A list of fields can go stale - a mission that adds a
//! twenty-second field would leave it silently unscored - so this counts.

use gearmaster_engine::stats::{Stats, When};
use gearmaster_oracle::s0;

#[test]
fn every_field_the_engine_classifies_is_one_the_surrogate_reads() {
    // A block with every field set draws one part per field.
    let all = Stats {
        health: 1, strength: 1, regen: 1, power: 1, armor: 1, mana: 1, mind: 1,
        mind_resist: 1, curse_resist: 1, physical_damage: 1, physical_resist: 1,
        physical_pierce: 1, physical_harden: 1, magic_damage: 1, magic_resist: 1,
        magic_pierce: 1, magic_harden: 1, reflect: 1, rage: 1, faith: 1, nature: 1,
    };
    assert_eq!(
        all.parts_when().len(),
        s0::classified(),
        "the engine classifies {} figures and the surrogate reads {} - a field \
         was added to Stats and s0.rs's FIELDS did not hear about it",
        all.parts_when().len(),
        s0::classified()
    );
}

#[test]
fn the_eight_that_are_handed_over_every_activation_are_found() {
    // T3's own count: eight of the twenty-one fields are per-activation. If
    // this moves, the surrogate is pricing rates as quantities again and the
    // packer's inner objective is wrong in the direction that matters.
    let all = Stats {
        health: 1, strength: 1, regen: 1, power: 1, armor: 1, mana: 1, mind: 1,
        mind_resist: 1, curse_resist: 1, physical_damage: 1, physical_resist: 1,
        physical_pierce: 1, physical_harden: 1, magic_damage: 1, magic_resist: 1,
        magic_pierce: 1, magic_harden: 1, reflect: 1, rage: 1, faith: 1, nature: 1,
    };
    let per_activation =
        all.parts_when().iter().filter(|(_, _, w)| *w == When::OnActivation).count();
    let damage = all.parts_when().iter().filter(|(_, _, w)| *w == When::Damage).count();
    assert_eq!(
        per_activation + damage,
        8,
        "eight of a Stats block is handed over on every activation; this reads \
         {} on-activation and {} damage",
        per_activation,
        damage
    );
}
