//! The gate, checked against the one it was ported from.
//!
//! `design/the-apprentice.md` §8 says to port `pack_francis`'s gate rather
//! than write a new one, because the gate is where four missions of balance
//! judgement are written down. A port is only a port if something checks it
//! against the original, so this reads the original's constants out of its
//! source and compares - which catches the case a hand-copied number always
//! eventually meets, where one of the two moves.

use gearmaster_oracle::gate::{
    self, band_for, off_curve, preset_holds, target_ms, BAND, CASINO_BAR_MS, FLAT_UNTIL, FLOOR_MS,
};
use gearmaster_oracle::Fight;

const ORIGINAL: &str = include_str!("../../engine/tests/pack_francis.rs");

fn constant(decl: &str) -> i64 {
    let at = ORIGINAL.find(decl).unwrap_or_else(|| panic!("`{}` moved", decl));
    let rest = &ORIGINAL[at..];
    let eq = rest.find('=').expect("a value");
    let digits: String = rest[eq..]
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '_')
        .filter(|c| *c != '_')
        .collect();
    digits.parse().expect("a number")
}

#[test]
fn every_constant_is_the_originals() {
    assert_eq!(FLOOR_MS as i64, constant("const FLOOR_MS: u32"), "the line's intercept");
    assert_eq!(FLAT_UNTIL as i64, constant("const FLAT_UNTIL: usize"), "the flat window");
    assert_eq!(
        CASINO_BAR_MS as i64,
        constant("const CASINO_BAR_MS: u32"),
        "the casino's bar"
    );
    // A float, so it is read as a string rather than through `constant`.
    assert!(
        ORIGINAL.contains("const BAND: f64 = 0.30;"),
        "the band moved; this port says {}",
        BAND
    );
}

#[test]
fn the_curve_is_the_same_curve_at_every_rung() {
    // The whole line, rung by rung, against the original's arithmetic written
    // out here. If either moves, this says which rung first.
    for rung in 0..50 {
        let want = if rung < 10 { 2_000 } else { 2_000 + 490 * (rung + 1 - 10) as u32 };
        assert_eq!(target_ms(rung), want, "rung {}", rung + 1);
    }
    assert_eq!(target_ms(0), 2_000, "rung 1 is two seconds");
    assert_eq!(target_ms(49), 21_600, "rung 50 is 21.6s, whose upper band edge is inside 30s");
    assert!(
        target_ms(49) as f64 * (1.0 + BAND) < 30_000.0,
        "the top of the band has to stay inside sudden death"
    );
}

#[test]
fn the_band_is_wider_while_the_curve_is_flat() {
    assert_eq!(band_for(0), 0.60);
    assert_eq!(band_for(9), 0.60);
    assert_eq!(band_for(10), BAND);
    assert_eq!(band_for(49), BAND);
}

fn beat(won: bool, ms: u32) -> Fight {
    Fight { won, ms, health_left: 0, enemy_health_left: 0, hurt: true, board_decided: won && ms < 30_000 }
}

#[test]
fn a_loss_is_infinitely_far_from_the_curve() {
    assert_eq!(off_curve(beat(false, 5_000), 20), f64::MAX);
    assert!(off_curve(beat(true, 2_000), 0) < 1e-9, "exactly on the line is zero off it");
}

#[test]
fn the_preset_corridor_says_what_the_original_says() {
    // A fight the preset used to win it must still win, and not take more than
    // twice as long. Deeper than the preset can reach it says nothing.
    assert!(preset_holds(beat(false, 9_000), beat(false, 9_000)), "a loss before says nothing");
    assert!(preset_holds(beat(true, 5_000), beat(true, 9_000)), "slower, and inside twice");
    assert!(!preset_holds(beat(true, 5_000), beat(true, 11_000)), "more than twice as long");
    assert!(!preset_holds(beat(true, 5_000), beat(false, 3_000)), "a win that became a loss");
}

#[test]
fn the_port_reads_the_same_fight_the_original_prints() {
    // Cog Priest, as it stands in `combat.rs`. The original packer's own
    // output for it reads
    //
    //     board want W8.0s W14.0s W14.0s W14.0s got W8.0s W9.0s W9.0s W9.0s
    //
    // where `want` is the shipped board and `got` is the candidate the search
    // was proposing. So the owner's board beats the **shipped** creature in
    // 14.0s at Medium against a 9.35s line - half a band outside it - and the
    // candidate at 9.0s is the one on the curve. A port that accepted the
    // shipped board would be a port that had lost the curve.
    //
    // Re-baseline by running the original:
    //   PACK_MONSTER="Cog Priest" cargo test --release -p gearmaster-engine \
    //     --test pack_francis pack -- --ignored --nocapture --exact
    use gearmaster_engine::combat::LADDER;
    use gearmaster_oracle::gate::{Gate, References, Verdict};
    use gearmaster_oracle::{Board, Oracle};

    let refs = References::standard();
    let oracle = Oracle::new();
    let rung = LADDER.iter().position(|s| s.name == "Cog Priest").expect("on the ladder");
    let spec = &LADDER[rung];
    let g = Gate { refs: &refs, rung, rank: spec.rank };
    let rows = g.rows(&oracle, spec);

    // Judged against itself: the shipped board is the incumbent, so every
    // corridor holds by construction and the only live question is the curve.
    let board = Board {
        gear: spec
            .gear
            .iter()
            .map(|&(name, s, x, y, r)| {
                (
                    gearmaster_engine::piece::CATALOG.iter().position(|d| d.name == name).unwrap(),
                    s,
                    x,
                    y,
                    r,
                )
            })
            .collect(),
        chunks: spec.items.to_vec(),
        rows: [0; 5],
    };
    // The eight figures the original prints, read by this port.
    let owner_medium = rows[2][1];
    assert!(owner_medium.won, "the owner's board beats the shipped Cog Priest");
    assert!(
        (owner_medium.ms as i64 - 14_000).abs() < 200,
        "the original prints W14.0s there; this port reads {:.1}s",
        owner_medium.ms as f64 / 1000.0
    );
    assert!(rows[3][1].won && (rows[3][1].ms as i64 - 6_400).abs() < 200, "the friend's 6.4s");
    assert!(!rows[0][1].won, "the four-piece board loses to it");
    assert!(!rows[1][1].won, "and so does the preset");

    let off = off_curve(owner_medium, rung);
    assert!(
        (off - 0.497).abs() < 0.01,
        "14.0s against a {} ms line is 0.497 off it; this port says {:.3}",
        target_ms(rung),
        off
    );
    let v = g.judge(&rows, &rows, &board);
    assert!(
        matches!(v, Verdict::OffCurve { .. }),
        "the shipped board is off the curve and the gate should say so: {:?}",
        v
    );
    assert!(off > gate::band_for(rung), "by more than the band allows");
}
