//! Combat: a fixed-timestep simulation where every assembled item runs its own
//! cooldown.
//!
//! There are no turns. The fight is stepped in [`TICK_MS`] slices and each item
//! fills its own bar independently, so a fast weapon really does swing more
//! often than a slow one. Nothing is random — the same loadout against the same
//! monster always produces the same log, which is what lets the tests assert on
//! exact numbers and lets the GUI replay a fight it did not simulate.

use crate::curse::{mind_damage_after_resist, CurseKind, Curses, STUN_CAP_MS, TICK_MS};
use crate::loadout::ItemProfile;
use crate::piece::{Action, Resource, SlotKind, Target, Trigger, Watched};
use crate::stats::Stats;

/// How often damage-over-time is summarised into the log.
pub const BURN_REPORT_MS: u32 = 1000;

/// A fight this long is called a draw, so a build that cannot finish the job
/// doesn't hang the simulation.
/// How long slow time spreads a hit over.
pub const SLOW_TIME_MS: u32 = 5000;

pub const MAX_DURATION_MS: u32 = 60_000;

/// When a fight that will not end starts ending itself.
///
/// Nothing happens for the first thirty seconds - a long fight is allowed to
/// be a long fight. Past that, both fighters take a share of their own maximum
/// health every second, and the share grows: one percent, then two, then
/// three. The total passes a hundred percent after fourteen seconds, so no
/// fight runs beyond about forty-four however much health or armour is in it,
/// which is the point.
///
/// It replaces a sixty-second cap that scored a draw as a loss. That rule made
/// every defensive option unplayable: armour buys survival, survival was not
/// victory, and a build that could out-last anything but out-damage nothing
/// lost anyway. Nothing here is dodgeable - the damage ignores armour and
/// resistance both, because a wall you can hide behind for ever is the thing
/// being fixed.
pub const SUDDEN_DEATH_MS: u32 = 30_000;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Side {
    Player,
    Enemy,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Player => Side::Enemy,
            Side::Enemy => Side::Player,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Side::Player => "You",
            Side::Enemy => "Enemy",
        }
    }
}

// ------------------------------------------------------------- monsters

/// One repeating attack belonging to a monster. Monsters use the same cooldown
/// machinery as the player's gear rather than a special case.
#[derive(Copy, Clone, Debug)]
pub struct MonsterAttack {
    pub name: &'static str,
    pub cooldown_ms: u32,
    pub damage: i32,
    pub mind: i32,
    pub armor: i32,
    /// Landed on the player each time this attack resolves.
    pub curse: Option<CurseKind>,
}

impl MonsterAttack {
    pub const fn hit(name: &'static str, cooldown_ms: u32, damage: i32) -> Self {
        MonsterAttack { name, cooldown_ms, damage, mind: 0, armor: 0, curse: None }
    }
    pub const fn cursing(
        name: &'static str,
        cooldown_ms: u32,
        damage: i32,
        curse: CurseKind,
    ) -> Self {
        MonsterAttack { name, cooldown_ms, damage, mind: 0, armor: 0, curse: Some(curse) }
    }
    pub const fn mind(name: &'static str, cooldown_ms: u32, mind: i32) -> Self {
        MonsterAttack { name, cooldown_ms, damage: 0, mind, armor: 0, curse: None }
    }
    pub const fn shielding(name: &'static str, cooldown_ms: u32, armor: i32) -> Self {
        MonsterAttack { name, cooldown_ms, damage: 0, mind: 0, armor, curse: None }
    }
}

/// Which silhouette to draw for a monster. Named rather than matched on the
/// monster's name, so a rename can't silently change what it looks like.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MonsterSprite {
    Rat,
    Warden,
    Gearwright,
    Toad,
    Archer,
    Golem,
    Wisp,
    Hound,
    Sentinel,
    Wraith,
    Idol,
    Fiend,
    King,
    // Added when the ladder grew to forty-nine and thirteen silhouettes were
    // being shared five ways. A creature you cannot tell from the last one is
    // a creature you have not really met.
    Francis,
    Marshal,
    Null,
    Lantern,
    Choir,
    Silence,
    Hourglass,
    Tallow,
    Weeping,
    Wedding,
    Twin,
    Mirror,
    Sootmother,
    Ashes,
    Crown,
    Drowned,
    Anvil,
    Parliament,
    Abbot,
    Gilt,
    Vermin,
    Behemoth,
    Cantor,
    Ember,
    Curator,
    Idiot,
    Rimefather,
    Slag,
    Obsidian,
    Gallows,
    CogPriest,
    RuinHound,
    Salt,
    Verdigris,
    March,
    Bells,
    Colossus,
}

/// One entry in a monster's loadout: `(component, slot, x, y, quarter turns)`.
pub type GearPlacement = (&'static str, SlotKind, u8, u8, u8);

/// What kind of fight this is.
///
/// Not decoration: rank decides how densely a creature is allowed to pack its
/// board (see `Rank::min_items_per_slot`), and whether beating it drops
/// something a shop will never sell you.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum Rank {
    #[default]
    Ordinary,
    Mini,
    Boss,
}

impl Rank {
    /// How many assembled items each slot this creature *uses* must hold.
    ///
    /// An ordinary creature is allowed a loose board. The named ones are not:
    /// a boss whose helmet holds one item is a boss you out-gear, and the
    /// whole point of locking items is that a board can hold more than the
    /// authoring tool used to be able to find.
    ///
    /// "Uses" rather than "has", which it used to be. A themed creature wears
    /// two slots and a themed hybrid three or four - that is what a theme is -
    /// so demanding density in all five demands that no creature have a theme.
    /// The density rule is about the slots a creature actually turns up
    /// wearing; how many of those there must be is `min_slots`.
    pub fn min_items_per_slot(self) -> usize {
        match self {
            Rank::Ordinary => 0,
            Rank::Mini => 2,
            Rank::Boss => 3,
        }
    }

    /// The same, for one slot.
    ///
    /// The weapon is one item whatever the rank. A creature carrying three
    /// swings three times a cooldown and no board can answer that - which the
    /// packer has enforced since it was written, while this rule asked a boss
    /// for three items in every slot it wears. Six of the ten named fights are
    /// given a weapon slot by their theme, so for six of ten the two rules
    /// could not both be satisfied; The Dreaming Idiot was already in that
    /// state, unnoticed, because the test that asks walks the ladder and it is
    /// an alternate.
    pub fn min_items_in(self, slot: SlotKind) -> usize {
        if slot == SlotKind::Weapon {
            self.min_items_per_slot().min(1)
        } else {
            self.min_items_per_slot()
        }
    }

    /// How many slots a named creature has to turn up wearing.
    ///
    /// Set by the themes rather than by taste. A mini-boss is a hybrid of its
    /// own cluster and the next, and the thinnest such pairing shares no slot
    /// and has none to spare: the two drainers at rungs 39 and 43 find nothing
    /// past rung 44 to hybridise with and wear their own two. A boss's two
    /// clusters always overlap by one, which is three. Anything below these is
    /// a named fight hiding in a corner of its board.
    pub fn min_slots(self) -> usize {
        match self {
            Rank::Ordinary => 0,
            Rank::Mini => 2,
            Rank::Boss => 3,
        }
    }

    pub fn is_named(self) -> bool {
        !matches!(self, Rank::Ordinary)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MonsterSpec {
    pub name: &'static str,
    /// Innate stats before gear: mostly just how much health it has.
    pub health: i32,
    /// Innate strength, which its weapons then scale.
    pub strength: i32,
    pub regen: i32,
    pub mind_resist: i32,
    pub curse_resist: i32,
    /// The two resistances most attacks answer to. Without these on the
    /// ladder, piercing and hardening would be inert: you would always be
    /// piercing nothing.
    pub physical_resist: i32,
    pub magic_resist: i32,
    /// Innate attacks — a rat's teeth, not equipment. Most of the ladder
    /// leaves this empty and fights with gear instead.
    pub attacks: &'static [MonsterAttack],
    /// Real components in real slots, assembled by the same rules the player
    /// plays by. This is what actually sets a monster's difficulty: to make one
    /// harder, give it better gear.
    pub gear: &'static [GearPlacement],
    /// Steps this monster's gear up or down its own kinds, on top of whatever
    /// the difficulty does. Negative means it fights in worse equipment than
    /// written at every setting.
    ///
    /// This is the dial for a monster that is out of step with its rung -
    /// preferable to rewriting its loadout, because the harder settings still
    /// climb from wherever it is put.
    ///
    /// Every monster sits at 0. Eight mid-ladder ones were once stepped down
    /// to soften a wall at rung 9, but the wall turned out to be the balance
    /// harness packing its builds too loosely, not the monsters. Move one off
    /// zero only with evidence from a densely packed profile.
    pub gear_offset: i32,
    /// Gold awarded for beating it.
    pub bounty: i32,
    pub sprite: MonsterSprite,
    /// Ordinary, mini-boss or boss. Defaults to ordinary, which is what the
    /// forty creatures that are neither stay at.
    pub rank: Rank,
    /// Components only this creature carries, and only it can drop. Empty for
    /// everything that is not named.
    pub drops: &'static [&'static str],
    /// How many pieces of `gear`, in order, make up each item.
    ///
    /// Needed only where a board is packed tightly enough that the pieces
    /// would otherwise negotiate: two items sitting flush merge into one
    /// over-full item unless each is locked before the next goes down. Empty
    /// means "work it out", which is right for the loose boards.
    pub items: &'static [usize],
}

/// The component this one becomes `step` rungs up its own kind.
///
/// Same kind and the same footprint, so the monster's layout still packs
/// exactly as authored - no re-solving, and a boss cannot end up with a hole
/// in its board because a swap was one cell too wide. Where a kind has no
/// better piece of that shape, the original stands.
pub fn stepped_component(name: &str, step: i32) -> &'static str {
    use crate::piece::CATALOG;
    let Some(here) = CATALOG.iter().find(|d| d.name == name) else { return "" };
    if step == 0 {
        return here.name;
    }
    let mut family: Vec<&'static crate::piece::PieceDef> = CATALOG
        .iter()
        .filter(|d| d.kind == here.kind && d.slot == here.slot && d.cells == here.cells)
        // Never step into gear that belongs to somebody.
        //
        // A trophy is off the scale for its slot by design, and stepping does
        // not know that: it sorts a footprint family by rating and takes the
        // next one up. On Hard that handed the Padded Base's family Francis's
        // coat - 2100 health - so the fourth creature on the ladder fought
        // with 2400 health instead of 475, and forty-five others were doing
        // the same thing. It was one piece until ten trophies were added, and
        // then it was everywhere.
        .filter(|d| !crate::piece::is_boss_only(d.name))
        // Quest rewards are earned, not stepped into, for the same reason
        // they are kept off the shelves.
        .filter(|d| !crate::piece::is_quest_reward(d.name))
        .collect();
    // Ordered by what a piece is worth to a *creature*, not to a shop.
    //
    // This sorted on `piece_rating`, which prices gear for a player who can
    // build a run around it. A drain rates well on that reasoning and does
    // nothing at all against a board banking no pools - so Francis's Insane
    // step traded a damage crest for Tithe Collector and came out easier than
    // his Hard step. A difficulty setting that lowers the difficulty is worse
    // than any mispricing it was correcting.
    family.sort_by(|a, b| {
        crate::rating::monster_value(a)
            .partial_cmp(&crate::rating::monster_value(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let Some(at) = family.iter().position(|d| d.name == here.name) else { return here.name };
    let want = (at as i32 + step).clamp(0, family.len() as i32 - 1) as usize;
    family[want].name
}

impl MonsterSpec {
    /// Lay this monster's gear out in real slots. Returned so the interface can
    /// draw an enemy's board exactly the way it draws yours.
    /// This monster's gear, stepped for a difficulty.
    pub fn gear_at(&self, difficulty: Difficulty) -> Vec<GearPlacement> {
        let step = difficulty.gear_step();
        self.gear
            .iter()
            .map(|&(name, slot, x, y, rot)| {
                (stepped_component(name, step + self.gear_offset), slot, x, y, rot)
            })
            .collect()
    }

    pub fn loadout(&self) -> (crate::piece::PieceRegistry, crate::loadout::Loadout) {
        self.loadout_at(Difficulty::Medium)
    }

    pub fn loadout_at(
        &self,
        difficulty: Difficulty,
    ) -> (crate::piece::PieceRegistry, crate::loadout::Loadout) {
        let gear = self.gear_at(difficulty);
        let mut reg = crate::piece::PieceRegistry::new();
        let mut loadout = crate::loadout::Loadout::new();
        // Seed names off the monster's own name so its gear is named too, and
        // named the same way every run.
        loadout.name_seed = self.name.bytes().fold(0xA5A5_u64, |a, b| {
            a.rotate_left(7) ^ b as u64
        });

        // Placed in item order, locking each one before the next goes down.
        //
        // The order matters and it is not cosmetic. An unlocked board
        // negotiates with itself: two items packed flush merge, or trade their
        // optional pieces to whichever core is nearest, and what comes out is
        // an over-full item that assembles into nothing. Locking each item as
        // it lands is what makes a tightly packed board hold - the same button
        // the player has, which the creatures use now too.
        let mut at = 0usize;
        let mut chunks: Vec<usize> = self.items.to_vec();
        if chunks.is_empty() {
            chunks = vec![gear.len()];
        }
        for take in chunks {
            let end = (at + take).min(gear.len());
            let mut touched: Vec<SlotKind> = Vec::new();
            for &(name, slot, x, y, rot) in &gear[at..end] {
                let Some(def) = crate::piece::CATALOG.iter().position(|d| d.name == name) else {
                    continue;
                };
                let id = reg.alloc(def);
                reg.set_rotation(id, rot);
                if loadout.can_place(&reg, id, slot, x, y).is_ok() {
                    loadout.slot_mut(slot).place(&reg, id, x, y);
                    if !touched.contains(&slot) {
                        touched.push(slot);
                    }
                }
            }
            for kind in touched {
                crate::loadout::lock_assembled_in(&mut loadout, &reg, kind);
            }
            at = end;
        }
        (reg, loadout)
    }

    /// Build this monster's loadout and reduce it to stats plus activation
    /// profiles — the exact pipeline the player's gear goes through.
    pub fn outfit(&self) -> (Stats, Vec<ItemProfile>) {
        self.outfit_at(Difficulty::Medium)
    }

    pub fn outfit_at(&self, difficulty: Difficulty) -> (Stats, Vec<ItemProfile>) {
        let (reg, loadout) = self.loadout_at(difficulty);
        let mut stats = loadout.total_stats(&reg);
        // `total_stats` starts from the player's baseline; swap in the
        // monster's own.
        stats.health = stats.health - crate::stats::BASE_HEALTH + self.health;
        // Swap the player's baseline strength for the monster's own.
        stats.strength = stats.strength - crate::stats::BASE_STRENGTH + self.strength;
        stats.regen += self.regen;
        stats.mind_resist += self.mind_resist;
        stats.curse_resist += self.curse_resist;
        stats.physical_resist += self.physical_resist;
        stats.magic_resist += self.magic_resist;

        // Past a point on the road, everything knows how to get through
        // armour, and past a further point it knows how to shrug off somebody
        // else's piercing.
        //
        // A rule rather than fifty hand-set numbers. Half the deep ladder was
        // swinging for two hundred physical with no piercing at all, so a
        // player who stacked one resistance simply stopped being hit - and the
        // defence triangle, which is most of what the late catalogue is about,
        // did nothing from either side. Written here so it stays true when the
        // ladder is renumbered, which has happened three times.
        let depth = LADDER.iter().position(|m| m.name == self.name).map(|i| i + 1);
        if let Some(rung) = depth {
            if rung > PIERCE_FROM {
                // Enough to matter against a build that has committed to one
                // resistance, and never enough to make committing pointless.
                let p = (15 + (rung - PIERCE_FROM) as i32 * 2).min(55);
                // Relevant to what it actually deals: there is no sense
                // piercing magic resistance with a club.
                let phys: i32 =
                    stats.physical_damage + stats.strength + stats.rage;
                let magic: i32 = stats.magic_damage;
                if phys > 0 {
                    stats.physical_pierce += p;
                }
                if magic > 0 {
                    stats.magic_pierce += p;
                }
            }
            if rung > HARDEN_FROM {
                let h = (10 + (rung - HARDEN_FROM) as i32 * 2).min(45);
                stats.physical_harden += h;
                stats.magic_harden += h;
            }
        }
        (stats, loadout.combat_items(&reg))
    }

    /// Which of its gear failed to assemble, if any. A monster whose loadout
    /// silently falls apart is a monster that does nothing.
    pub fn unassembled(&self) -> Vec<String> {
        let mut missing = Vec::new();
        for &(name, _, _, _, _) in self.gear {
            if !crate::piece::CATALOG.iter().any(|d| d.name == name) {
                missing.push(format!("{}: no such component", name));
            }
        }
        // Built through `loadout_at`, not by re-placing the gear here. The two
        // are not the same board: `loadout_at` locks each item as it lands,
        // and on a tightly packed board that is the difference between three
        // items and one over-full one that assembles into nothing. Checking a
        // board the creature never fights in is worse than not checking.
        let (reg, loadout) = self.loadout();
        for kind in SlotKind::ALL {
            for item in loadout.report(&reg, kind).items {
                if !item.assembled {
                    missing.push(format!("{} item: {}", kind.name(), item.status));
                }
            }
        }
        missing
    }
}

/// How much harder than a baseline run this is.
///
/// The scale is what the player picks - 1x, 3x, 9x, 27x - and it is the
/// monster's total effectiveness that gets multiplied, not any one stat.
/// Splitting it evenly between staying alive and hitting back means each side
/// takes the square root, so their product is the factor you chose: Insane is
/// a monster about 5.2 times tougher and 5.2 times deadlier, which is 27 times
/// the fight.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Insane,
}

impl Difficulty {
    pub const ALL: &'static [Difficulty] =
        &[Difficulty::Easy, Difficulty::Medium, Difficulty::Hard, Difficulty::Insane];

    /// The advertised multiple: how many times as effective the opposition is.
    pub fn factor(self) -> f32 {
        match self {
            Difficulty::Easy => 0.5,
            Difficulty::Medium => 1.0,
            Difficulty::Hard => 3.0,
            Difficulty::Insane => 9.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "EASY",
            Difficulty::Medium => "MEDIUM",
            Difficulty::Hard => "HARD",
            Difficulty::Insane => "INSANE",
        }
    }

    pub fn label(self) -> String {
        let f = self.factor();
        if (f - f.round()).abs() < 0.01 {
            format!("{}x", f as i32)
        } else {
            format!("{}x", f)
        }
    }

    /// Medium is the way the game is meant to be played; the others are set
    /// against it.
    pub fn is_default(self) -> bool {
        matches!(self, Difficulty::Medium)
    }

    /// How far up its own kind each of a monster's components is swapped.
    ///
    /// This is where most of a difficulty setting now lives. Medium is the
    /// gear as written; Hard and Insane trade each component for a better one
    /// of the same kind, and Easy trades down. A Bog Toad on Insane is not the
    /// Medium toad with bigger numbers - it is a toad in better armour.
    pub fn gear_step(self) -> i32 {
        match self {
            Difficulty::Easy => -1,
            Difficulty::Medium => 0,
            Difficulty::Hard => 1,
            Difficulty::Insane => 2,
        }
    }

    /// What is left for raw stats to carry, once gear has done its part.
    ///
    /// Deliberately small. Multiplying health and damage is the crude lever;
    /// it is kept only as a floor, because a component has no better version
    /// to swap to at the top of its kind and the setting still has to mean
    /// something.
    pub fn each_way(self) -> f32 {
        self.factor().powf(0.25)
    }

    /// Standing bonuses the opposition gets on top of the raw scaling. These
    /// are the prototype for class passives: a named rule that edits a
    /// combatant's stats once, at the start of the fight.
    pub fn passives(self) -> &'static [Passive] {
        match self {
            Difficulty::Easy => &[],
            Difficulty::Medium => &[Passive::Hardened],
            Difficulty::Hard => &[Passive::Hardened, Passive::Warded],
            Difficulty::Insane => &[Passive::Hardened, Passive::Warded, Passive::Relentless],
        }
    }
}

/// What kind of harm an attack is, so the matching defences apply.
///
/// There is no untyped option on purpose. Every number a piece of gear deals
/// is physical, magic, or mind, which is what makes resistance worth buying:
/// a defence that half the game ignored would be a coin flip at the shop.
/// Curse burn is the one thing that still bypasses all of it, and it answers
/// to curse resistance instead.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum DamageType {
    #[default]
    Physical,
    Magic,
}

impl DamageType {
    pub fn name(self) -> &'static str {
        match self {
            DamageType::Physical => "physical damage",
            DamageType::Magic => "magic damage",
        }
    }
}

/// A standing rule that edits a combatant before the fight starts.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Passive {
    /// Knits itself back together: regeneration every second.
    Hardened,
    /// Turns aside the mind and the curse alike.
    Warded,
    /// Never stops coming: everything it does lands sooner.
    Relentless,
}

impl Passive {
    pub fn name(self) -> &'static str {
        match self {
            Passive::Hardened => "Hardened",
            Passive::Warded => "Warded",
            Passive::Relentless => "Relentless",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Passive::Hardened => "heals 4 a second",
            Passive::Warded => "shrugs off 40% of mind and curses, 20% of blows and spells",
            Passive::Relentless => "all its gear comes round a quarter sooner",
        }
    }
}

/// The original opponent, named because several tests predate the ladder.
pub const RUST_GOLEM: MonsterSpec = MonsterSpec {
    name: "Rust Golem",
    health: 300,
    strength: 13,
    regen: 0,
    mind_resist: 0,
    physical_resist: 7,
        magic_resist: 7,
        curse_resist: 0,
    attacks: &[],
    gear: &[
        ("Chained Codex", SlotKind::Weapon, 0, 0, 1),
        ("Kingsblood Ink", SlotKind::Weapon, 3, 0, 0),
        ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
        ("Bloodstone Bead", SlotKind::Weapon, 4, 1, 1),
    ],
    gear_offset: 0,
    bounty: 10,
    sprite: MonsterSprite::Golem,
    rank: Rank::Ordinary,
    drops: &[],
    items: &[4],
};

/// The monster ladder, easiest first.
///
/// Difficulty is set by what each one is *wearing*, not by hand-tuned numbers:
/// they buy from the same catalogue and assemble by the same rules. Making a
/// monster harder means giving it better gear.
/// What a spell costs to cast at full strength, and what it lands for when
/// there is nothing to pay with.
///
/// An unpaid spell is not cancelled - it still fires, which matters, because a
/// build that runs dry should get weaker rather than stop.
pub const SPELL_MANA_COST: i32 = 3;
pub const WEAK_CAST_PCT: i32 = 45;

/// What a paid cast lands for.
///
/// Playtesters found spells universally weak and crystal balls not worth the
/// room they take. The reason was that paying for a spell bought you nothing
/// except not being weakened - the ceiling was the number printed on the
/// piece, and that number had to compete with a blade that swings for it
/// every time and never asks for mana. So paying now doubles the cast. The
/// shop price is unchanged on purpose: the point is to make casters worth
/// their slot, not to make them cost more.
pub const EMPOWERED_CAST_PCT: i32 = 200;

/// How many of its spells a crystal ball casts each time it comes round.
///
/// Two, always. A class can raise it; nothing lowers it.
pub const BALL_VOICES: u32 = 2;

/// The rung past which everything on the road pierces, and past which it also
/// hardens. Both are exclusive: rung 30 does not, rung 31 does.
pub const PIERCE_FROM: usize = 30;
pub const HARDEN_FROM: usize = 40;

pub const LADDER: &[MonsterSpec] = &[
    MonsterSpec {
        name: "Cave Rat",
        health: 55,
        strength: 2,
        regen: 0,
        mind_resist: 0,
        physical_resist: 1,
        magic_resist: 1,
        curse_resist: 0,
        // No gear at all — it just has teeth.
        attacks: &[MonsterAttack::hit("bite", 900, 4)],
        gear: &[],
        gear_offset: 0,
        bounty: 6,
        sprite: MonsterSprite::Rat,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[],
    },
    MonsterSpec {
        name: "Bog Toad",
        health: 110,
        strength: 5,
        regen: 1,
        mind_resist: 0,
        physical_resist: 2,
        magic_resist: 2,
        curse_resist: 0,
        attacks: &[],
        // Fast hands, and something sharp.
        gear: &[
            ("Godsteel Haft", SlotKind::Weapon, 0, 0, 1),
            ("Iron Blade", SlotKind::Weapon, 0, 1, 1),
            ("Worldweave Material", SlotKind::Gloves, 0, 0, 0),
            ("Bramble Mold", SlotKind::Gloves, 3, 0, 0),
        ],
        gear_offset: 0,
        bounty: 8,
    sprite: MonsterSprite::Toad,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 2],
    },
    MonsterSpec {
        name: "Bone Archer",
        health: 120,
        strength: 5,
        regen: 0,
        mind_resist: 0,
        physical_resist: 3,
        magic_resist: 3,
        curse_resist: 0,
        attacks: &[],
        // Fast hands, and something sharp.
        gear: &[
            ("Emberheart Orb", SlotKind::Weapon, 0, 0, 0),
            ("Cometfall", SlotKind::Weapon, 3, 0, 0),
            ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
            ("Cometfall", SlotKind::Weapon, 4, 1, 1),
        ],
        gear_offset: 0,
        bounty: 9,
    sprite: MonsterSprite::Archer,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4],
    },
    RUST_GOLEM,
    MonsterSpec {
        name: "Frost Wisp",
        health: 150,
        strength: 6,
        regen: 0,
        mind_resist: 0,
        physical_resist: 3,
        magic_resist: 3,
        curse_resist: 25,
        attacks: &[],
        // Fast hands, and something sharp.
        gear: &[
            ("Emberheart Orb", SlotKind::Weapon, 0, 0, 0),
            ("Unmaking", SlotKind::Weapon, 3, 0, 1),
            ("Hollow Lance", SlotKind::Weapon, 5, 0, 0),
            ("Worldweave Material", SlotKind::Gloves, 0, 0, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 0, 0),
        ],
        gear_offset: 0,
        bounty: 12,
    sprite: MonsterSprite::Wisp,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[3, 2],
    },
    MonsterSpec {
        name: "Plague Hound",
        health: 190,
        strength: 8,
        regen: 0,
        mind_resist: 0,
        physical_resist: 4,
        magic_resist: 4,
        curse_resist: 0,
        attacks: &[],
        // Fast hands, and something sharp.
        gear: &[
            ("Emberheart Orb", SlotKind::Weapon, 0, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 3, 0, 0),
            ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
            ("Sanctified Material", SlotKind::Gloves, 0, 0, 1),
            ("Sovereign Mold", SlotKind::Gloves, 2, 0, 0),
        ],
        gear_offset: 0,
        bounty: 14,
    sprite: MonsterSprite::Hound,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[3, 2],
    },
    MonsterSpec {
        name: "The Iron Warden",
        health: 340,
        strength: 14,
        regen: 2,
        mind_resist: 20,
        physical_resist: 8,
        magic_resist: 8,
        curse_resist: 20,
        attacks: &[],
        // Halfway up the ladder, and the first opponent whose armour is the
        // point: every one of the 48 chest cells is covered, by three separate
        // chestpieces, so it soaks far more than anything before it. The rest
        // of its gear is deliberately ordinary - one weapon, one glove, one
        // Made to be hit, and it hits back harder than it hits.
        gear: &[
            ("Zealot's Haft", SlotKind::Weapon, 0, 0, 1),
            ("Gluttonous Fang", SlotKind::Weapon, 3, 0, 1),
            ("Ossuary Frame", SlotKind::Helmet, 0, 0, 1),
            ("Lonely Plating", SlotKind::Helmet, 2, 0, 0),
            ("Grove Base", SlotKind::Chest, 0, 0, 0),
            ("Vast Tapestry", SlotKind::Chest, 2, 1, 1),
        ],
        gear_offset: 0,
        bounty: 22,
        sprite: MonsterSprite::Warden,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 2, 2],
    },
    MonsterSpec {
        name: "Iron Sentinel",
        health: 240,
        strength: 10,
        regen: 0,
        mind_resist: 0,
        physical_resist: 6,
        magic_resist: 6,
        curse_resist: 0,
        attacks: &[],
        // Made to be hit, and it hits back harder than it hits.
        gear: &[
            ("Zealot's Haft", SlotKind::Weapon, 0, 0, 1),
            ("Gluttonous Fang", SlotKind::Weapon, 3, 0, 1),
            ("Ossuary Frame", SlotKind::Helmet, 0, 0, 1),
            ("Lonely Plating", SlotKind::Helmet, 2, 0, 0),
            ("Heartwood Base", SlotKind::Chest, 0, 0, 0),
            ("Vast Tapestry", SlotKind::Chest, 0, 2, 0),
        ],
        gear_offset: 0,
        bounty: 24,
    sprite: MonsterSprite::Sentinel,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 2, 2],
    },
    MonsterSpec {
        name: "Whisperling",
        health: 160,
        strength: 7,
        regen: 0,
        mind_resist: 0,
        physical_resist: 4,
        magic_resist: 4,
        curse_resist: 0,
        attacks: &[],
        // Made to be hit, and it hits back harder than it hits.
        gear: &[
            ("Scrying Orb", SlotKind::Weapon, 0, 0, 0),
            ("Echo Sigil", SlotKind::Weapon, 3, 0, 0),
            ("Mirror Ward", SlotKind::Weapon, 4, 1, 0),
            ("Rime Nova", SlotKind::Weapon, 0, 3, 0),
            ("Bone Frame", SlotKind::Helmet, 0, 0, 0),
            ("Braced Plating", SlotKind::Helmet, 3, 0, 0),
            ("Bone Frame", SlotKind::Helmet, 0, 1, 2),
            ("Braced Plating", SlotKind::Helmet, 3, 2, 0),
            ("Hexweave Shroud", SlotKind::Chest, 0, 0, 0),
            ("Ironbark Layer", SlotKind::Chest, 3, 0, 0),
            ("Hexweave Shroud", SlotKind::Chest, 3, 2, 0),
            ("Scale Layer", SlotKind::Chest, 0, 3, 0),
            ("Steel Material", SlotKind::Greaves, 0, 0, 1),
            ("Warded Sabatons", SlotKind::Greaves, 3, 0, 3),
            ("Steel Material", SlotKind::Greaves, 4, 1, 0),
            ("Warded Sabatons", SlotKind::Greaves, 1, 2, 3),
        ],
        gear_offset: 0,
        bounty: 26,
    sprite: MonsterSprite::Wraith,
        rank: Rank::Mini,
        drops: &["Asker's Monocle"],
        items: &[4, 2, 2, 2, 2, 2, 2],
    },
    MonsterSpec {
        name: "Warded Idol",
        health: 280,
        strength: 12,
        regen: 2,
        mind_resist: 0,
        physical_resist: 7,
        magic_resist: 7,
        curse_resist: 55,
        attacks: &[],
        // Made to be hit, and it hits back harder than it hits.
        gear: &[
            ("Executioner's Haft", SlotKind::Weapon, 0, 0, 1),
            ("Iron Blade", SlotKind::Weapon, 0, 1, 1),
            ("Reliquary Frame of Nine", SlotKind::Helmet, 0, 0, 0),
            ("Warded Plating", SlotKind::Helmet, 3, 0, 0),
            ("Voidsilk Base", SlotKind::Chest, 0, 0, 0),
            ("Vast Tapestry", SlotKind::Chest, 0, 3, 0),
        ],
        gear_offset: 0,
        bounty: 30,
    sprite: MonsterSprite::Idol,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 2, 2],
    },
    MonsterSpec {
        name: "Mirror Fiend",
        health: 250,
        strength: 11,
        regen: 0,
        mind_resist: 45,
        physical_resist: 6,
        magic_resist: 6,
        curse_resist: 20,
        attacks: &[],
        gear: &[
            ("Grand Grimoire", SlotKind::Weapon, 0, 0, 0),
            ("Starlit Ink", SlotKind::Weapon, 3, 0, 0),
            ("Unmaking", SlotKind::Weapon, 2, 1, 3),
            ("Ossuary Frame", SlotKind::Helmet, 0, 0, 1),
            ("Scarred Plating", SlotKind::Helmet, 2, 0, 0),
            ("Heartwood Base", SlotKind::Chest, 0, 0, 0),
            ("Verdant Weave", SlotKind::Chest, 3, 0, 0),
        ],
        gear_offset: 0,
        bounty: 34,
    sprite: MonsterSprite::Fiend,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[3, 2, 2],
    },
    MonsterSpec {
        name: "Rust Colossus",
        // gear rating 47
        health: 800,
        strength: 28,
        regen: 2,
        mind_resist: 20,
        physical_resist: 20,
        magic_resist: 15,
        curse_resist: 20,
        attacks: &[],
        gear: &[
            ("Mage's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Bloomcap", SlotKind::Helmet, 3, 0, 0),
            ("Hide Base", SlotKind::Chest, 0, 0, 0),
            ("Berserker's Plate", SlotKind::Chest, 3, 0, 0),
            ("Mage's Wrapping", SlotKind::Gloves, 0, 0, 0),
            ("Featherweight Mold", SlotKind::Gloves, 2, 0, 0),
            ("Mage's Sandals", SlotKind::Greaves, 0, 0, 0),
            ("Striding Mold", SlotKind::Greaves, 2, 0, 0),
            ("Apprentice's Primer", SlotKind::Weapon, 0, 0, 0),
            ("Bloodletter's Ink", SlotKind::Weapon, 2, 0, 0),
            ("Warding Sigil", SlotKind::Weapon, 4, 0, 0),
            ("Duskweave Material", SlotKind::Gloves, 2, 1, 0),
            ("Empowering Mold", SlotKind::Gloves, 2, 2, 0),
        ],
        gear_offset: 0,
        bounty: 44,
        sprite: MonsterSprite::Colossus,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[],
    },
    MonsterSpec {
        name: "Ashen Marshal",
        // gear rating 99
        health: 930,
        strength: 31,
        regen: 2,
        mind_resist: 23,
        physical_resist: 23,
        magic_resist: 18,
        curse_resist: 23,
        attacks: &[],
        gear: &[
            ("Apprentice's Primer", SlotKind::Weapon, 0, 0, 0),
            ("Starlit Ink", SlotKind::Weapon, 2, 0, 0),
            ("Unmaking", SlotKind::Weapon, 4, 0, 2),
            ("Ossuary Frame", SlotKind::Helmet, 0, 0, 1),
            ("Scarred Plating", SlotKind::Helmet, 2, 0, 0),
            ("Overflow Plate", SlotKind::Helmet, 4, 0, 0),
            ("Heartwood Base", SlotKind::Chest, 0, 0, 0),
            ("Berserker's Plate", SlotKind::Chest, 3, 0, 0),
        ],
        gear_offset: 0,
        bounty: 75,
        sprite: MonsterSprite::Marshal,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[3, 3, 2],
    },
    MonsterSpec {
        name: "Grave Chorus",
        // gear rating 154
        health: 1060,
        strength: 34,
        regen: 2,
        mind_resist: 26,
        physical_resist: 26,
        magic_resist: 21,
        curse_resist: 26,
        attacks: &[],
        gear: &[
            ("Chained Codex", SlotKind::Weapon, 0, 0, 1),
            ("Kingsblood Ink", SlotKind::Weapon, 3, 0, 0),
            ("Cometfall", SlotKind::Weapon, 3, 1, 2),
            ("Bloodstone Bead", SlotKind::Weapon, 0, 2, 0),
            ("Rootwoven Material", SlotKind::Greaves, 0, 0, 0),
            ("Pilgrim Sole", SlotKind::Greaves, 3, 0, 0),
            ("Consecrated Plating", SlotKind::Greaves, 0, 1, 0),
            ("Plaguewalkers", SlotKind::Greaves, 2, 2, 0),
            ("Pilgrim Sole", SlotKind::Greaves, 4, 2, 0),
            ("Consecrated Plating", SlotKind::Greaves, 0, 3, 0),
        ],
        gear_offset: 0,
        bounty: 80,
        sprite: MonsterSprite::Choir,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 3, 3],
    },
    MonsterSpec {
        name: "The Hollow King",
        health: 400,
        strength: 18,
        regen: 3,
        mind_resist: 30,
        physical_resist: 10,
        magic_resist: 5,
        curse_resist: 30,
        attacks: &[],
        // It does not need to reach you.
        gear: &[
            ("Timeworn Orb", SlotKind::Weapon, 0, 0, 0),
            ("Unmaking", SlotKind::Weapon, 3, 0, 1),
            ("Last Rite", SlotKind::Weapon, 4, 0, 1),
            ("Emberburst", SlotKind::Weapon, 1, 1, 0),
            ("Ember Alignment", SlotKind::Weapon, 0, 2, 3),
            ("Tallykeeper's Weave", SlotKind::Greaves, 0, 0, 0),
            ("Witherroot", SlotKind::Greaves, 2, 0, 3),
            ("Warmed Material", SlotKind::Greaves, 4, 0, 0),
            ("Witherroot", SlotKind::Greaves, 2, 1, 1),
            ("Tallykeeper's Weave", SlotKind::Greaves, 0, 2, 0),
            ("Witherroot", SlotKind::Greaves, 2, 3, 3),
            ("Thornweald Grip", SlotKind::Gloves, 0, 0, 0),
            ("Bramble Mold", SlotKind::Gloves, 2, 0, 0),
            ("Tallykeeper's Weave", SlotKind::Gloves, 4, 0, 0),
            ("Flaying Mold", SlotKind::Gloves, 2, 1, 2),
            ("Tallykeeper's Weave", SlotKind::Gloves, 0, 2, 0),
            ("Bramble Mold", SlotKind::Gloves, 2, 3, 0),
        ],
        gear_offset: 0,
        bounty: 89,
    sprite: MonsterSprite::King,
        rank: Rank::Boss,
        drops: &["Henpeck's Cell Keys"],
        items: &[5, 2, 2, 2, 2, 2, 2],
    },
    // The buyer Henpeck names as he goes down. The player has been buying
    // gear off this one since rung one without ever asking where a shop that
    // size gets its stock.
    MonsterSpec {
        name: "The Curator",
        health: 640,
        strength: 24,
        regen: 4,
        mind_resist: 34,
        physical_resist: 16,
        magic_resist: 18,
        curse_resist: 32,
        attacks: &[],
        gear: &[
            ("Emberheart Orb", SlotKind::Weapon, 0, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 3, 0, 0),
            ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
            ("Ember Alignment", SlotKind::Weapon, 3, 1, 1),
            ("Sanctified Material", SlotKind::Greaves, 0, 0, 1),
            ("Gravewalker Mold", SlotKind::Greaves, 2, 0, 0),
            ("Consecrated Plating", SlotKind::Greaves, 1, 1, 0),
            ("Rootwoven Material", SlotKind::Greaves, 3, 1, 0),
            ("Pilgrim Sole", SlotKind::Greaves, 3, 2, 0),
            ("Consecrated Plating", SlotKind::Greaves, 1, 3, 0),
            ("Rootwoven Material", SlotKind::Greaves, 0, 2, 1),
            ("Gravewalker Mold", SlotKind::Greaves, 0, 5, 0),
        ],
        gear_offset: 0,
        bounty: 93,
        sprite: MonsterSprite::Curator,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 3, 3, 2],
    },
    MonsterSpec {
        name: "Salt Idol",
        // gear rating 208
        health: 1190,
        strength: 37,
        regen: 3,
        mind_resist: 29,
        physical_resist: 29,
        magic_resist: 24,
        curse_resist: 29,
        attacks: &[],
        gear: &[
            ("Zealot's Haft", SlotKind::Weapon, 0, 0, 1),
            ("Gluttonous Fang", SlotKind::Weapon, 3, 0, 1),
            ("Sunderer", SlotKind::Weapon, 0, 1, 1),
            ("Bloodstone Bead", SlotKind::Weapon, 5, 0, 1),
            ("Oathstone Bead", SlotKind::Weapon, 4, 1, 1),
            ("Rootwoven Material", SlotKind::Greaves, 0, 0, 0),
            ("Pilgrim Sole", SlotKind::Greaves, 3, 0, 0),
            ("Consecrated Plating", SlotKind::Greaves, 0, 1, 0),
            ("Ashwoven Material", SlotKind::Greaves, 5, 0, 1),
            ("Witherroot", SlotKind::Greaves, 3, 2, 3),
            ("Consecrated Plating", SlotKind::Greaves, 1, 3, 0),
            ("Rootwoven Material", SlotKind::Greaves, 0, 3, 1),
            ("Pilgrim Sole", SlotKind::Greaves, 1, 5, 0),
        ],
        gear_offset: 0,
        bounty: 98,
        sprite: MonsterSprite::Salt,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[5, 3, 3, 2],
    },
    MonsterSpec {
        name: "Pale Twin",
        // gear rating 259
        health: 1320,
        strength: 40,
        regen: 3,
        mind_resist: 32,
        physical_resist: 33,
        magic_resist: 28,
        curse_resist: 32,
        attacks: &[],
        gear: &[
            ("Emberheart Orb", SlotKind::Weapon, 0, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 3, 0, 0),
            ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
            ("Cometfall", SlotKind::Weapon, 3, 1, 0),
            ("Ember Alignment", SlotKind::Weapon, 4, 2, 2),
            ("Sanctified Material", SlotKind::Greaves, 0, 0, 1),
            ("Pilgrim Sole", SlotKind::Greaves, 2, 0, 0),
            ("Plaguewalkers", SlotKind::Greaves, 4, 0, 0),
            ("Pilgrim Sole", SlotKind::Greaves, 3, 2, 0),
            ("Consecrated Plating", SlotKind::Greaves, 1, 2, 0),
            ("Sanctified Material", SlotKind::Greaves, 0, 3, 0),
            ("Pilgrim Sole", SlotKind::Greaves, 2, 4, 0),
            ("Consecrated Plating", SlotKind::Greaves, 4, 4, 0),
        ],
        gear_offset: 0,
        bounty: 107,
        sprite: MonsterSprite::Twin,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[5, 2, 3, 3],
    },
    MonsterSpec {
        name: "Ruin Hound",
        // gear rating 311
        health: 1450,
        strength: 43,
        regen: 3,
        mind_resist: 35,
        physical_resist: 36,
        magic_resist: 31,
        curse_resist: 35,
        attacks: &[],
        gear: &[
            ("Emberheart Orb", SlotKind::Weapon, 0, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 3, 0, 0),
            ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
            ("Ember Alignment", SlotKind::Weapon, 3, 1, 1),
            ("Rootwoven Material", SlotKind::Greaves, 0, 0, 0),
            ("Witherroot", SlotKind::Greaves, 3, 0, 3),
            ("Rootwoven Material", SlotKind::Greaves, 0, 1, 0),
            ("Gravewalker Mold", SlotKind::Greaves, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Greaves, 3, 2, 0),
            ("Rootwoven Material", SlotKind::Greaves, 5, 0, 1),
            ("Anchored Sole", SlotKind::Greaves, 4, 3, 3),
            ("Consecrated Plating", SlotKind::Greaves, 2, 4, 0),
            ("Rootwoven Material", SlotKind::Greaves, 0, 3, 0),
            ("Pilgrim Sole", SlotKind::Greaves, 0, 4, 0),
            ("Consecrated Plating", SlotKind::Greaves, 0, 6, 0),
        ],
        gear_offset: 0,
        bounty: 116,
        sprite: MonsterSprite::RuinHound,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 2, 3, 3, 3],
    },
    MonsterSpec {
        name: "Bone Cantor",
        // gear rating 368
        health: 1580,
        strength: 46,
        regen: 4,
        mind_resist: 38,
        physical_resist: 39,
        magic_resist: 34,
        curse_resist: 38,
        attacks: &[],
        gear: &[
            ("Emberheart Orb", SlotKind::Weapon, 0, 0, 0),
            ("Kingsbane", SlotKind::Weapon, 3, 0, 0),
            ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
            ("Cometfall", SlotKind::Weapon, 4, 1, 1),
            ("Ember Alignment", SlotKind::Weapon, 0, 3, 0),
            ("Sevenleague Boots", SlotKind::Greaves, 0, 0, 1),
            ("Worldstrider Sole", SlotKind::Greaves, 3, 0, 0),
            ("Consecrated Plating", SlotKind::Greaves, 0, 2, 0),
            ("Rootwoven Material", SlotKind::Greaves, 2, 2, 0),
            ("Pilgrim Sole", SlotKind::Greaves, 2, 3, 0),
            ("Tallykeeper's Weave", SlotKind::Gloves, 0, 0, 0),
            ("Sovereign Mold", SlotKind::Gloves, 2, 0, 0),
            ("Unshod Signet", SlotKind::Gloves, 5, 0, 0),
            ("Ring of Embers", SlotKind::Gloves, 2, 1, 0),
            ("Ashwoven Material", SlotKind::Gloves, 0, 2, 0),
            ("Sovereign Mold", SlotKind::Gloves, 3, 1, 2),
        ],
        gear_offset: 0,
        bounty: 125,
        sprite: MonsterSprite::Cantor,
        rank: Rank::Mini,
        drops: &["Toolwright's Grip"],
        items: &[5, 3, 2, 4, 2],
    },
    MonsterSpec {
        name: "Ember Wisp",
        // gear rating 420
        health: 1710,
        strength: 49,
        regen: 4,
        mind_resist: 41,
        physical_resist: 42,
        magic_resist: 37,
        curse_resist: 41,
        attacks: &[],
        gear: &[
            ("Sanctified Material", SlotKind::Greaves, 0, 0, 1),
            ("Sapling Mold", SlotKind::Greaves, 2, 0, 1),
            ("Sanctified Material", SlotKind::Greaves, 4, 0, 1),
            ("Sapling Mold", SlotKind::Greaves, 2, 1, 1),
            ("Braced Plating", SlotKind::Greaves, 1, 2, 0),
            ("Sanctified Material", SlotKind::Greaves, 4, 1, 3),
            ("Grave-Iron Mold", SlotKind::Greaves, 3, 2, 2),
            ("Sanctified Material", SlotKind::Greaves, 0, 3, 0),
            ("Sapling Mold", SlotKind::Greaves, 2, 4, 1),
            ("Mage's Wrapping", SlotKind::Gloves, 0, 0, 0),
            ("Deft Mold", SlotKind::Gloves, 2, 0, 0),
            ("Sanctified Material", SlotKind::Gloves, 4, 0, 1),
            ("Deft Mold", SlotKind::Gloves, 2, 1, 0),
            ("Iron Band", SlotKind::Gloves, 5, 1, 0),
            ("Oathring", SlotKind::Gloves, 2, 2, 0),
            ("Mage's Wrapping", SlotKind::Gloves, 0, 2, 0),
            ("Featherweight Mold", SlotKind::Gloves, 2, 3, 0),
        ],
        gear_offset: 0,
        bounty: 134,
        sprite: MonsterSprite::Ember,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 3, 2, 2, 2, 4, 2],
    },
    MonsterSpec {
        name: "Slag Warden",
        // gear rating 480
        health: 1840,
        strength: 52,
        regen: 4,
        mind_resist: 44,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 44,
        attacks: &[],
        gear: &[
            ("Sanctified Material", SlotKind::Greaves, 0, 0, 1),
            ("Sapling Mold", SlotKind::Greaves, 2, 0, 1),
            ("Witch's Claw", SlotKind::Greaves, 4, 0, 0),
            ("Runner's Mold", SlotKind::Greaves, 2, 1, 0),
            ("Braced Plating", SlotKind::Greaves, 0, 2, 0),
            ("Sanctified Material", SlotKind::Greaves, 4, 2, 3),
            ("Sapling Mold", SlotKind::Greaves, 2, 3, 1),
            ("Braced Plating", SlotKind::Greaves, 1, 4, 0),
            ("Witch's Stilts", SlotKind::Gloves, 0, 0, 1),
            ("Deft Mold", SlotKind::Gloves, 3, 0, 0),
            ("Sanctified Material", SlotKind::Gloves, 4, 0, 3),
            ("Deft Mold", SlotKind::Gloves, 2, 1, 0),
            ("Sanctified Material", SlotKind::Gloves, 0, 1, 3),
            ("Deft Mold", SlotKind::Gloves, 2, 2, 0),
            ("Tin Band", SlotKind::Gloves, 4, 2, 0),
            ("Oathring", SlotKind::Gloves, 5, 2, 0),
            ("Sanctified Material", SlotKind::Gloves, 0, 3, 1),
            ("Deft Mold", SlotKind::Gloves, 2, 3, 0),
        ],
        gear_offset: 0,
        bounty: 143,
        sprite: MonsterSprite::Slag,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 3, 3, 2, 2, 4, 2],
    },
    MonsterSpec {
        name: "The Gearwright",
        health: 720,
        strength: 26,
        regen: 4,
        mind_resist: 40,
        physical_resist: 18,
        magic_resist: 13,
        curse_resist: 40,
        attacks: &[],
        // The end of the ladder: every slot filled with the best-rated legal
        // item the catalogue allows, found by the packing search in
        // It takes your time before it takes anything else.
        gear: &[
            ("Kingmaker Hilt", SlotKind::Weapon, 0, 0, 0),
            ("Sunderer", SlotKind::Weapon, 3, 0, 1),
            ("Oathstone Bead", SlotKind::Weapon, 0, 1, 1),
            ("Fury Sigil", SlotKind::Weapon, 2, 1, 0),
            ("Sevenleague Boots", SlotKind::Gloves, 0, 0, 1),
            ("Quickfinger Mold", SlotKind::Gloves, 3, 0, 1),
            ("Sevenleague Boots", SlotKind::Gloves, 4, 1, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 2, 2, 1),
            ("Titan's Grip", SlotKind::Greaves, 0, 0, 0),
            ("Worldstrider Sole", SlotKind::Greaves, 3, 0, 0),
            ("Consecrated Plating", SlotKind::Greaves, 0, 2, 0),
            ("Sevenleague Boots", SlotKind::Greaves, 2, 2, 1),
            ("Worldstrider Sole", SlotKind::Greaves, 0, 4, 0),
            ("Steel Frame", SlotKind::Helmet, 0, 0, 0),
            ("Scrying Lens", SlotKind::Helmet, 3, 0, 0),
            ("Bloomcap", SlotKind::Helmet, 3, 1, 3),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 3, 0),
        ],
        gear_offset: 0,
        bounty: 152,
        sprite: MonsterSprite::Gearwright,
        rank: Rank::Mini,
        drops: &["Kaklon's Patent"],
        items: &[4, 2, 2, 3, 2, 3, 2],
    },
    // ---- past the Gearwright ----
    //
    // Twenty more, climbing steadily. Each wears a loadout built from layouts
    // already verified to assemble, so the ladder can grow without every new
    // rung needing the packing search run over it again.
    MonsterSpec {
        name: "Crowned Hollow",
        // gear rating 532
        health: 1970,
        strength: 55,
        regen: 5,
        mind_resist: 47,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 47,
        attacks: &[],
        gear: &[
            ("Leaden Tome", SlotKind::Weapon, 0, 0, 0),
            ("Deepwater Ink", SlotKind::Weapon, 3, 0, 1),
            ("Slash and Burn", SlotKind::Weapon, 3, 1, 0),
            ("Sevenleague Boots", SlotKind::Gloves, 0, 0, 1),
            ("Quickfinger Mold", SlotKind::Gloves, 3, 0, 1),
            ("Unshod Signet", SlotKind::Gloves, 5, 0, 0),
            ("Ring of Roots", SlotKind::Gloves, 4, 1, 0),
            ("Sevenleague Boots", SlotKind::Gloves, 0, 2, 1),
            ("Gripping Mold", SlotKind::Gloves, 3, 2, 0),
            ("Sanctified Material", SlotKind::Greaves, 0, 0, 1),
            ("Trailworn Sole", SlotKind::Greaves, 2, 0, 1),
            ("Consecrated Plating", SlotKind::Greaves, 4, 0, 0),
            ("Rootwoven Material", SlotKind::Greaves, 1, 1, 1),
            ("Sapling Mold", SlotKind::Greaves, 2, 2, 1),
            ("Consecrated Plating", SlotKind::Greaves, 4, 2, 0),
            ("Witch's Hat", SlotKind::Helmet, 0, 0, 2),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Warding Plate", SlotKind::Helmet, 2, 2, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 4, 2, 1),
            ("Consecrated Plating", SlotKind::Helmet, 2, 4, 0),
        ],
        gear_offset: 0,
        bounty: 161,
        sprite: MonsterSprite::Crown,
        rank: Rank::Mini,
        drops: &["Eighth Ray Crown"],
        items: &[3, 4, 2, 3, 3, 3, 2],
    },
    MonsterSpec {
        name: "Cog Priest",
        // gear rating 588
        health: 2100,
        strength: 58,
        regen: 5,
        mind_resist: 50,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 50,
        attacks: &[],
        gear: &[
            ("Rootwoven Material", SlotKind::Greaves, 0, 0, 0),
            ("Worldstrider Sole", SlotKind::Greaves, 3, 0, 0),
            ("Tallykeeper's Weave", SlotKind::Greaves, 0, 1, 0),
            ("Worldstrider Sole", SlotKind::Greaves, 2, 1, 2),
            ("Consecrated Plating", SlotKind::Greaves, 0, 3, 0),
            ("Sevenleague Boots", SlotKind::Greaves, 2, 3, 1),
            ("Worldstrider Sole", SlotKind::Greaves, 0, 5, 0),
            ("Consecrated Plating", SlotKind::Greaves, 3, 5, 0),
            ("Sanctified Material", SlotKind::Gloves, 0, 0, 1),
            ("Gauntlet Mold", SlotKind::Gloves, 2, 0, 1),
            ("Unshod Signet", SlotKind::Gloves, 5, 0, 0),
            ("Ring of Roots", SlotKind::Gloves, 1, 1, 0),
            ("Sanctified Material", SlotKind::Gloves, 3, 1, 1),
            ("Deft Mold", SlotKind::Gloves, 5, 1, 1),
            ("Unshod Signet", SlotKind::Gloves, 2, 2, 0),
            ("Ring of Roots", SlotKind::Gloves, 1, 2, 0),
            ("Sanctified Material", SlotKind::Gloves, 0, 2, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 2, 3, 1),
            ("Witch's Stilts", SlotKind::Gloves, 4, 2, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 2, 4, 3),
        ],
        gear_offset: 0,
        bounty: 170,
        sprite: MonsterSprite::CogPriest,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 3, 3, 4, 4, 2, 2],
    },
    MonsterSpec {
        name: "Mire Behemoth",
        // gear rating 642
        health: 2230,
        strength: 61,
        regen: 5,
        mind_resist: 53,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 53,
        attacks: &[],
        gear: &[
            ("Sanctified Material", SlotKind::Greaves, 0, 0, 1),
            ("Treadmill Sole", SlotKind::Greaves, 2, 0, 0),
            ("Witch's Stilts", SlotKind::Greaves, 0, 1, 3),
            ("Pilgrim Sole", SlotKind::Greaves, 3, 2, 0),
            ("Consecrated Plating", SlotKind::Greaves, 0, 3, 0),
            ("Sevenleague Boots", SlotKind::Greaves, 2, 4, 1),
            ("Worldstrider Sole", SlotKind::Greaves, 0, 5, 2),
            ("Consecrated Plating", SlotKind::Greaves, 3, 6, 0),
            ("Sevenleague Boots", SlotKind::Gloves, 0, 0, 1),
            ("Quickfinger Mold", SlotKind::Gloves, 3, 0, 1),
            ("Unshod Signet", SlotKind::Gloves, 5, 0, 0),
            ("Ring of Roots", SlotKind::Gloves, 4, 1, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 2, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 3, 2, 1),
            ("Unshod Signet", SlotKind::Gloves, 5, 2, 0),
            ("Unshod Signet", SlotKind::Gloves, 5, 1, 0),
            ("Sevenleague Boots", SlotKind::Gloves, 0, 3, 1),
            ("Quickfinger Mold", SlotKind::Gloves, 3, 3, 3),
            ("Unshod Signet", SlotKind::Gloves, 5, 3, 0),
            ("Oathring", SlotKind::Gloves, 5, 4, 0),
            ("Tallykeeper's Weave", SlotKind::Gloves, 0, 5, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 2, 5, 1),
        ],
        gear_offset: 0,
        bounty: 179,
        sprite: MonsterSprite::Behemoth,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 3, 3, 4, 4, 4, 2],
    },
    MonsterSpec {
        name: "Vermin Sovereign",
        // gear rating 695
        health: 2360,
        strength: 64,
        regen: 6,
        mind_resist: 56,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 56,
        attacks: &[],
        gear: &[
            ("Witch's Stilts", SlotKind::Greaves, 0, 0, 1),
            ("Worldstrider Sole", SlotKind::Greaves, 3, 0, 0),
            ("Consecrated Plating", SlotKind::Greaves, 1, 1, 0),
            ("Sanctified Material", SlotKind::Greaves, 3, 1, 0),
            ("Worldstrider Sole", SlotKind::Greaves, 1, 3, 0),
            ("Consecrated Plating", SlotKind::Greaves, 4, 3, 0),
            ("Sanctified Material", SlotKind::Greaves, 0, 3, 0),
            ("Sapling Mold", SlotKind::Greaves, 0, 5, 1),
            ("Rootwoven Material", SlotKind::Greaves, 3, 4, 1),
            ("Worldstrider Sole", SlotKind::Greaves, 1, 5, 1),
            ("Consecrated Plating", SlotKind::Greaves, 4, 5, 0),
            ("Sevenleague Boots", SlotKind::Gloves, 0, 0, 1),
            ("Quickfinger Mold", SlotKind::Gloves, 3, 0, 1),
            ("Unshod Signet", SlotKind::Gloves, 5, 0, 0),
            ("Ring of Roots", SlotKind::Gloves, 4, 1, 0),
            ("Warmed Material", SlotKind::Gloves, 0, 2, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 2, 2, 1),
            ("Warmed Material", SlotKind::Gloves, 4, 2, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 2, 3, 3),
            ("Unshod Signet", SlotKind::Gloves, 5, 1, 0),
            ("Iron Band", SlotKind::Gloves, 1, 4, 0),
            ("Sanctified Material", SlotKind::Gloves, 0, 4, 0),
            ("Gauntlet Mold", SlotKind::Gloves, 2, 4, 3),
        ],
        gear_offset: 0,
        bounty: 188,
        sprite: MonsterSprite::Vermin,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[3, 3, 2, 3, 4, 2, 4, 2],
    },
    MonsterSpec {
        name: "Obsidian Colossus",
        // gear rating 739
        health: 2490,
        strength: 67,
        regen: 6,
        mind_resist: 59,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 59,
        attacks: &[],
        gear: &[
            ("Sevenleague Boots", SlotKind::Greaves, 0, 0, 1),
            ("Worldstrider Sole", SlotKind::Greaves, 3, 0, 0),
            ("Consecrated Plating", SlotKind::Greaves, 0, 2, 0),
            ("Warmed Material", SlotKind::Greaves, 2, 2, 0),
            ("Trailworn Sole", SlotKind::Greaves, 4, 1, 3),
            ("Consecrated Plating", SlotKind::Greaves, 4, 3, 0),
            ("Sanctified Material", SlotKind::Greaves, 0, 4, 1),
            ("Trailworn Sole", SlotKind::Greaves, 2, 4, 1),
            ("Consecrated Plating", SlotKind::Greaves, 3, 5, 0),
            ("Sanctified Material", SlotKind::Greaves, 0, 5, 3),
            ("Zealot's Sole", SlotKind::Greaves, 0, 7, 1),
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 3, 0, 1),
            ("Unshod Signet", SlotKind::Gloves, 5, 0, 0),
            ("Oathring", SlotKind::Gloves, 0, 1, 0),
            ("Sevenleague Boots", SlotKind::Gloves, 1, 1, 0),
            ("Quickfinger Mold", SlotKind::Gloves, 3, 1, 3),
            ("Tin Band", SlotKind::Gloves, 5, 1, 0),
            ("Ring of Roots", SlotKind::Gloves, 0, 2, 0),
            ("Sevenleague Boots", SlotKind::Gloves, 3, 3, 1),
            ("Quickfinger Mold", SlotKind::Gloves, 1, 4, 1),
            ("Unshod Signet", SlotKind::Gloves, 5, 2, 0),
            ("Ring of Roots", SlotKind::Gloves, 0, 4, 0),
        ],
        gear_offset: 0,
        bounty: 197,
        sprite: MonsterSprite::Obsidian,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[3, 3, 3, 2, 4, 4, 4],
    },
    MonsterSpec {
        name: "Null Sentinel",
        // gear rating 809
        health: 2620,
        strength: 70,
        regen: 6,
        mind_resist: 62,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 62,
        attacks: &[],
        gear: &[
            ("Leaden Tome", SlotKind::Weapon, 0, 0, 0),
            ("Kingsblood Ink", SlotKind::Weapon, 3, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 4, 1, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 0, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 2, 0),
            ("Scrying Lens", SlotKind::Helmet, 0, 4, 0),
            ("Martyr's Crest", SlotKind::Helmet, 3, 4, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 5, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 5, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 5, 1),
            ("Forked Crest", SlotKind::Helmet, 0, 7, 0),
        ],
        gear_offset: 0,
        bounty: 206,
        sprite: MonsterSprite::Null,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[3, 3, 4, 4],
    },
    MonsterSpec {
        name: "Silence",
        // gear rating 861
        health: 2750,
        strength: 73,
        regen: 7,
        mind_resist: 65,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 65,
        attacks: &[],
        gear: &[
            ("Grovemind Orb", SlotKind::Weapon, 0, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 3, 0, 0),
            ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
            ("Rootwork Alignment", SlotKind::Weapon, 3, 1, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 0, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 2, 0),
            ("Swiftplate", SlotKind::Helmet, 3, 2, 0),
            ("Scrying Lens", SlotKind::Helmet, 0, 4, 0),
            ("Martyr's Crest", SlotKind::Helmet, 4, 3, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 5, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 6, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 4, 1),
        ],
        gear_offset: 0,
        bounty: 215,
        sprite: MonsterSprite::Silence,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 3, 4, 3],
    },
    MonsterSpec {
        name: "Weeping Idol",
        // gear rating 907
        health: 2880,
        strength: 76,
        regen: 7,
        mind_resist: 68,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 68,
        attacks: &[],
        gear: &[
            ("Orb of the Nine", SlotKind::Weapon, 0, 0, 0),
            ("Unmaking", SlotKind::Weapon, 3, 0, 1),
            ("Kingsbane", SlotKind::Weapon, 4, 0, 1),
            ("Cometfall", SlotKind::Weapon, 0, 2, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Swiftplate", SlotKind::Helmet, 4, 1, 2),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 3, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 4, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 5, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 6, 0),
            ("Sanctified Material", SlotKind::Gloves, 0, 0, 1),
            ("Flaying Mold", SlotKind::Gloves, 2, 0, 0),
            ("Grasping Ring", SlotKind::Gloves, 4, 0, 0),
            ("Deepdraught Ring", SlotKind::Gloves, 5, 0, 0),
            ("Titan's Grip", SlotKind::Gloves, 3, 1, 0),
            ("Flaying Mold", SlotKind::Gloves, 1, 1, 3),
            ("Titan's Grip", SlotKind::Gloves, 0, 3, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 3, 0),
            ("Unshod Signet", SlotKind::Gloves, 0, 2, 0),
            ("Deepdraught Ring", SlotKind::Gloves, 5, 3, 0),
            ("Titan's Grip", SlotKind::Gloves, 4, 4, 1),
            ("Flaying Mold", SlotKind::Gloves, 2, 5, 0),
            ("Mage's Sandals", SlotKind::Gloves, 0, 5, 0),
            ("Flaying Mold", SlotKind::Gloves, 0, 6, 2),
        ],
        gear_offset: 0,
        bounty: 224,
        sprite: MonsterSprite::Weeping,
        rank: Rank::Boss,
        drops: &["The Seeker's Tears"],
        items: &[4, 3, 2, 3, 4, 2, 4, 2, 2],
    },
    MonsterSpec {
        name: "The Long Mirror",
        // gear rating 933
        health: 3010,
        strength: 79,
        regen: 7,
        mind_resist: 70,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 70,
        attacks: &[],
        gear: &[
            ("Grovemind Orb", SlotKind::Weapon, 0, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 3, 0, 0),
            ("Hollow Lance", SlotKind::Weapon, 0, 2, 1),
            ("Starfall", SlotKind::Weapon, 3, 1, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Martyr's Crest", SlotKind::Helmet, 5, 0, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 2, 0),
            ("Bronze Frame", SlotKind::Helmet, 0, 4, 0),
            ("Consecrated Plating", SlotKind::Helmet, 2, 4, 0),
            ("Martyr's Crest", SlotKind::Helmet, 4, 4, 1),
            ("Bronze Frame", SlotKind::Helmet, 0, 6, 0),
            ("Consecrated Plating", SlotKind::Helmet, 2, 6, 0),
            ("Bloomcap", SlotKind::Helmet, 4, 6, 1),
        ],
        gear_offset: 0,
        bounty: 233,
        sprite: MonsterSprite::Mirror,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 3, 2, 3, 3],
    },
    MonsterSpec {
        name: "Iron Abbot",
        // gear rating 949
        health: 3140,
        strength: 82,
        regen: 8,
        mind_resist: 70,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 70,
        attacks: &[],
        gear: &[
            ("Kingmaker Hilt", SlotKind::Weapon, 0, 0, 0),
            ("Arcane Splinter", SlotKind::Weapon, 3, 0, 1),
            ("Arcane Splinter", SlotKind::Weapon, 4, 0, 3),
            ("Oathstone Bead", SlotKind::Weapon, 0, 1, 1),
            ("Fury Sigil", SlotKind::Weapon, 2, 1, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 0, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 2, 0),
            ("Scrying Lens", SlotKind::Helmet, 0, 4, 0),
            ("Martyr's Crest", SlotKind::Helmet, 3, 4, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 5, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 5, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 5, 1),
            ("Martyr's Crest", SlotKind::Helmet, 0, 7, 0),
        ],
        gear_offset: 0,
        bounty: 242,
        sprite: MonsterSprite::Abbot,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[5, 3, 4, 4],
    },
    MonsterSpec {
        name: "The Last Gearwright",
        // gear rating 956
        health: 3270,
        strength: 85,
        regen: 8,
        mind_resist: 70,
        physical_resist: 45,
        magic_resist: 40,
        curse_resist: 70,
        attacks: &[],
        gear: &[
            ("Grovemind Orb", SlotKind::Weapon, 0, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 3, 0, 0),
            ("Starfall", SlotKind::Weapon, 3, 1, 0),
            ("Cometfall", SlotKind::Weapon, 0, 2, 0),
            ("Rootwork Alignment", SlotKind::Weapon, 3, 2, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 0, 1),
            ("Martyr's Crest", SlotKind::Helmet, 0, 2, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 3, 2, 1),
            ("Consecrated Plating", SlotKind::Helmet, 1, 3, 0),
            ("Martyr's Crest", SlotKind::Helmet, 0, 3, 1),
            ("Bone Frame", SlotKind::Helmet, 3, 4, 2),
            ("Consecrated Plating", SlotKind::Helmet, 1, 5, 0),
            ("Swiftplate", SlotKind::Helmet, 0, 6, 3),
            ("Martyr's Crest", SlotKind::Helmet, 3, 6, 0),
        ],
        gear_offset: 0,
        bounty: 251,
        sprite: MonsterSprite::Gearwright,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[5, 4, 3, 4],
    },
    MonsterSpec {
        name: "Rimefather",
        health: 3480,
        strength: 92,
        regen: 8,
        mind_resist: 70,
        physical_resist: 48,
        magic_resist: 44,
        curse_resist: 70,
        attacks: &[],
        gear: &[
            ("Kingmaker Hilt", SlotKind::Weapon, 0, 0, 0),
            ("Arcane Splinter", SlotKind::Weapon, 3, 0, 1),
            ("Arcane Splinter", SlotKind::Weapon, 4, 0, 3),
            ("Oathstone Bead", SlotKind::Weapon, 0, 1, 1),
            ("Fury Sigil", SlotKind::Weapon, 2, 1, 0),
            ("Steel Frame", SlotKind::Helmet, 0, 0, 0),
            ("Bloomcap", SlotKind::Helmet, 3, 0, 3),
            ("Martyr's Crest", SlotKind::Helmet, 5, 0, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 2, 0),
            ("Swiftplate", SlotKind::Helmet, 4, 3, 2),
            ("Martyr's Crest", SlotKind::Helmet, 0, 4, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 5, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 5, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 5, 1),
            ("Martyr's Crest", SlotKind::Helmet, 0, 7, 0),
        ],
        gear_offset: 0,
        bounty: 262,
        sprite: MonsterSprite::Rimefather,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[5, 3, 4, 4],
    },
    MonsterSpec {
        name: "The Tallow Saint",
        health: 3690,
        strength: 96,
        regen: 9,
        mind_resist: 72,
        physical_resist: 50,
        magic_resist: 46,
        curse_resist: 72,
        attacks: &[],
        gear: &[
            ("Grovemind Orb", SlotKind::Weapon, 0, 0, 0),
            ("Slash and Burn", SlotKind::Weapon, 3, 0, 0),
            ("Sunder", SlotKind::Weapon, 3, 1, 0),
            ("Starfall", SlotKind::Weapon, 0, 2, 0),
            ("Rootwork Alignment", SlotKind::Weapon, 3, 2, 0),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Bronze Frame", SlotKind::Helmet, 0, 2, 0),
            ("Swiftplate", SlotKind::Helmet, 2, 2, 0),
            ("Scrying Lens", SlotKind::Helmet, 4, 2, 1),
            ("Martyr's Crest", SlotKind::Helmet, 5, 0, 1),
            ("Stormcaught Frame", SlotKind::Helmet, 0, 4, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 5, 0),
            ("Scrying Lens", SlotKind::Helmet, 5, 3, 1),
            ("Bone Frame", SlotKind::Helmet, 0, 6, 0),
            ("Scrying Lens", SlotKind::Helmet, 1, 7, 0),
        ],
        gear_offset: 0,
        bounty: 273,
        sprite: MonsterSprite::Tallow,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[5, 2, 4, 3, 2],
    },
    MonsterSpec {
        name: "Hollowmarch",
        health: 3910,
        strength: 101,
        regen: 9,
        mind_resist: 74,
        physical_resist: 52,
        magic_resist: 48,
        curse_resist: 74,
        attacks: &[],
        gear: &[
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 0, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 1, 0),
            ("Grasping Ring", SlotKind::Gloves, 5, 0, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 2, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 1, 2),
            ("Seal of the Deep", SlotKind::Gloves, 5, 1, 1),
            ("Blightfinger", SlotKind::Gloves, 2, 1, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 3, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 3, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 4, 0),
            ("Flaying Mold", SlotKind::Gloves, 0, 5, 0),
            ("Rootwoven Material", SlotKind::Gloves, 5, 3, 1),
            ("Flaying Mold", SlotKind::Gloves, 3, 4, 2),
            ("Mage's Wrapping", SlotKind::Gloves, 1, 6, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 6, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Swiftplate", SlotKind::Helmet, 3, 0, 0),
            ("Mirrored Visor", SlotKind::Helmet, 4, 0, 1),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 2, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 4, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 4, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 6, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 6, 0),
        ],
        gear_offset: 0,
        bounty: 284,
        sprite: MonsterSprite::March,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 4, 2, 2, 2, 2, 3, 2, 2, 2],
    },
    MonsterSpec {
        name: "The Iron Choir",
        health: 4140,
        strength: 106,
        regen: 10,
        mind_resist: 76,
        physical_resist: 54,
        magic_resist: 50,
        curse_resist: 76,
        attacks: &[],
        gear: &[
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 0, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 1, 0),
            ("Flaying Mold", SlotKind::Gloves, 0, 2, 0),
            ("Seal of the Deep", SlotKind::Gloves, 2, 2, 0),
            ("Grasping Ring", SlotKind::Gloves, 4, 2, 0),
            ("Rootwoven Material", SlotKind::Gloves, 5, 0, 1),
            ("Flaying Mold", SlotKind::Gloves, 4, 3, 0),
            ("Grasping Ring", SlotKind::Gloves, 4, 1, 0),
            ("Grasping Ring", SlotKind::Gloves, 3, 3, 0),
            ("Rootwoven Material", SlotKind::Gloves, 1, 3, 1),
            ("Flaying Mold", SlotKind::Gloves, 2, 3, 3),
            ("Seal of the Deep", SlotKind::Gloves, 0, 4, 1),
            ("Grasping Ring", SlotKind::Gloves, 2, 5, 0),
            ("Rootwoven Material", SlotKind::Gloves, 3, 5, 0),
            ("Flaying Mold", SlotKind::Gloves, 2, 6, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 2, 0),
            ("Crown of the Deep", SlotKind::Helmet, 4, 1, 1),
            ("Overseer's Circlet", SlotKind::Helmet, 2, 3, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 3, 3),
            ("Mirrored Visor", SlotKind::Helmet, 1, 5, 0),
            ("Warlord's Crest", SlotKind::Helmet, 4, 4, 3),
            ("Overseer's Circlet", SlotKind::Helmet, 3, 6, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 6, 2),
        ],
        gear_offset: 0,
        bounty: 295,
        sprite: MonsterSprite::Bells,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[2, 4, 4, 4, 2, 4, 4, 2],
    },
    MonsterSpec {
        name: "Gallowglass",
        health: 4380,
        strength: 112,
        regen: 10,
        mind_resist: 78,
        physical_resist: 56,
        magic_resist: 52,
        curse_resist: 78,
        attacks: &[],
        gear: &[
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Hexer's Tally", SlotKind::Gloves, 3, 0, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 1, 0),
            ("Grasping Ring", SlotKind::Gloves, 5, 0, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 2, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 1, 2),
            ("Rootwoven Material", SlotKind::Gloves, 5, 1, 1),
            ("Flaying Mold", SlotKind::Gloves, 3, 3, 0),
            ("Seal of the Deep", SlotKind::Gloves, 1, 3, 0),
            ("Grasping Ring", SlotKind::Gloves, 0, 3, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 4, 0),
            ("Flaying Mold", SlotKind::Gloves, 0, 5, 0),
            ("Grasping Ring", SlotKind::Gloves, 2, 5, 0),
            ("Grasping Ring", SlotKind::Gloves, 3, 5, 0),
            ("Rootwoven Material", SlotKind::Gloves, 4, 4, 1),
            ("Flaying Mold", SlotKind::Gloves, 2, 6, 0),
            ("Grasping Ring", SlotKind::Gloves, 5, 4, 0),
            ("Grasping Ring", SlotKind::Gloves, 5, 5, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 2, 0),
            ("Tithe Collector", SlotKind::Helmet, 5, 0, 1),
            ("Overseer's Circlet", SlotKind::Helmet, 3, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 2, 4, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 4, 1),
            ("Mirrored Visor", SlotKind::Helmet, 2, 6, 0),
            ("Mirrored Visor", SlotKind::Helmet, 4, 4, 1),
        ],
        gear_offset: 0,
        bounty: 306,
        sprite: MonsterSprite::Gallows,
        rank: Rank::Mini,
        drops: &["Assassin's Hemline"],
        items: &[4, 2, 4, 4, 4, 4, 2, 3],
    },
    MonsterSpec {
        name: "The Rust Parliament",
        health: 4640,
        strength: 118,
        regen: 11,
        mind_resist: 80,
        physical_resist: 58,
        magic_resist: 54,
        curse_resist: 80,
        attacks: &[],
        gear: &[
            ("Mage's Wrapping", SlotKind::Gloves, 0, 0, 0),
            ("Flaying Mold", SlotKind::Gloves, 2, 0, 0),
            ("Seal of the Deep", SlotKind::Gloves, 4, 0, 0),
            ("Grasping Ring", SlotKind::Gloves, 3, 1, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 2, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 1, 2),
            ("Rootwoven Material", SlotKind::Gloves, 5, 1, 1),
            ("Flaying Mold", SlotKind::Gloves, 3, 3, 0),
            ("Seal of the Deep", SlotKind::Gloves, 1, 3, 0),
            ("Grasping Ring", SlotKind::Gloves, 0, 3, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 4, 0),
            ("Flaying Mold", SlotKind::Gloves, 0, 5, 0),
            ("Mage's Wrapping", SlotKind::Gloves, 4, 4, 0),
            ("Flaying Mold", SlotKind::Gloves, 2, 5, 0),
            ("Seal of the Deep", SlotKind::Gloves, 3, 6, 0),
            ("Grasping Ring", SlotKind::Gloves, 1, 6, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 7, 0),
            ("Gauntlet Mold", SlotKind::Gloves, 3, 6, 3),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Hastening Crest", SlotKind::Helmet, 5, 0, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 2, 0),
            ("Bloomcap", SlotKind::Helmet, 4, 3, 1),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 4, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 5, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 6, 0),
            ("Scrying Lens", SlotKind::Helmet, 3, 7, 0),
        ],
        gear_offset: 0,
        bounty: 317,
        sprite: MonsterSprite::Parliament,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 2, 4, 2, 4, 2, 3, 3, 2, 2],
    },
    MonsterSpec {
        name: "Sootmother",
        health: 4910,
        strength: 124,
        regen: 11,
        mind_resist: 82,
        physical_resist: 60,
        magic_resist: 56,
        curse_resist: 82,
        attacks: &[],
        gear: &[
            ("Sanctified Material", SlotKind::Gloves, 0, 0, 1),
            ("Gauntlet Mold", SlotKind::Gloves, 2, 0, 1),
            ("Seal of the Deep", SlotKind::Gloves, 5, 0, 1),
            ("Grasping Ring", SlotKind::Gloves, 1, 1, 0),
            ("Mage's Wrapping", SlotKind::Gloves, 3, 1, 0),
            ("Flaying Mold", SlotKind::Gloves, 1, 2, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 2, 1),
            ("Grasping Ring", SlotKind::Gloves, 5, 2, 0),
            ("Sanctified Material", SlotKind::Gloves, 2, 3, 1),
            ("Flaying Mold", SlotKind::Gloves, 4, 3, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 4, 1),
            ("Flaying Mold", SlotKind::Gloves, 1, 4, 3),
            ("Rootwoven Material", SlotKind::Gloves, 3, 4, 1),
            ("Flaying Mold", SlotKind::Gloves, 4, 4, 2),
            ("Grasping Ring", SlotKind::Gloves, 2, 6, 0),
            ("Grasping Ring", SlotKind::Gloves, 1, 6, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 7, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 6, 2),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Swiftplate", SlotKind::Helmet, 3, 0, 0),
            ("Mirrored Visor", SlotKind::Helmet, 4, 0, 1),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 2, 0),
            ("Bloomcap", SlotKind::Helmet, 3, 2, 3),
            ("Bloomcap", SlotKind::Helmet, 4, 3, 3),
            ("Crown of the Deep", SlotKind::Helmet, 0, 4, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 2, 5, 0),
            ("Consecrated Plating", SlotKind::Helmet, 0, 6, 0),
            ("Warlord's Crest", SlotKind::Helmet, 4, 6, 3),
        ],
        gear_offset: 0,
        bounty: 328,
        sprite: MonsterSprite::Sootmother,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 4, 2, 2, 4, 2, 3, 4, 3],
    },
    MonsterSpec {
        name: "The Quiet Hour",
        health: 5190,
        strength: 131,
        regen: 12,
        mind_resist: 84,
        physical_resist: 62,
        magic_resist: 58,
        curse_resist: 84,
        attacks: &[],
        gear: &[
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 0, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 1, 0),
            ("Blightfinger", SlotKind::Gloves, 5, 0, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 2, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 1, 2),
            ("Seal of the Deep", SlotKind::Gloves, 5, 1, 1),
            ("Grasping Ring", SlotKind::Gloves, 2, 1, 0),
            ("Spun Material", SlotKind::Gloves, 0, 3, 1),
            ("Flaying Mold", SlotKind::Gloves, 2, 3, 0),
            ("Seal of the Deep", SlotKind::Gloves, 4, 3, 0),
            ("Blightfinger", SlotKind::Gloves, 1, 4, 0),
            ("Rootwoven Material", SlotKind::Gloves, 3, 4, 0),
            ("Gauntlet Mold", SlotKind::Gloves, 1, 5, 1),
            ("Rootwoven Material", SlotKind::Gloves, 0, 5, 1),
            ("Flaying Mold", SlotKind::Gloves, 1, 6, 2),
            ("Mage's Wrapping", SlotKind::Gloves, 4, 5, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 6, 3),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 2, 0),
            ("Warlord's Crest", SlotKind::Helmet, 4, 1, 3),
            ("Overseer's Circlet", SlotKind::Helmet, 2, 3, 0),
            ("Consecrated Plating", SlotKind::Helmet, 0, 4, 0),
            ("Mirrored Visor", SlotKind::Helmet, 4, 4, 1),
            ("Overseer's Circlet", SlotKind::Helmet, 2, 5, 1),
            ("Consecrated Plating", SlotKind::Helmet, 0, 6, 0),
            ("Warlord's Crest", SlotKind::Helmet, 4, 6, 0),
        ],
        gear_offset: 0,
        bounty: 339,
        sprite: MonsterSprite::Hourglass,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 4, 4, 2, 2, 2, 4, 3, 3],
    },
    MonsterSpec {
        name: "Verdigris",
        health: 5490,
        strength: 138,
        regen: 12,
        mind_resist: 86,
        physical_resist: 63,
        magic_resist: 60,
        curse_resist: 86,
        attacks: &[],
        gear: &[
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 0, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 1, 0),
            ("Grasping Ring", SlotKind::Gloves, 5, 0, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 2, 0),
            ("Throttling Mold", SlotKind::Gloves, 3, 2, 0),
            ("Grasping Ring", SlotKind::Gloves, 2, 1, 0),
            ("Grasping Ring", SlotKind::Gloves, 4, 1, 0),
            ("Rootwoven Material", SlotKind::Gloves, 5, 1, 1),
            ("Throttling Mold", SlotKind::Gloves, 4, 4, 0),
            ("Seal of the Deep", SlotKind::Gloves, 2, 4, 0),
            ("Grasping Ring", SlotKind::Gloves, 2, 3, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 3, 1),
            ("Flaying Mold", SlotKind::Gloves, 1, 4, 3),
            ("Seal of the Deep", SlotKind::Gloves, 3, 5, 1),
            ("Grasping Ring", SlotKind::Gloves, 1, 3, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 6, 0),
            ("Gauntlet Mold", SlotKind::Gloves, 2, 6, 3),
            ("Seal of the Deep", SlotKind::Gloves, 0, 7, 0),
            ("Grasping Ring", SlotKind::Gloves, 5, 6, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 2, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 3, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 2, 4, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 3, 3),
            ("Overseer's Circlet", SlotKind::Helmet, 4, 4, 1),
            ("Consecrated Plating", SlotKind::Helmet, 2, 6, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 5, 1),
        ],
        gear_offset: 0,
        bounty: 350,
        sprite: MonsterSprite::Verdigris,
        rank: Rank::Mini,
        drops: &["Handman's Peel"],
        items: &[4, 4, 4, 4, 4, 3, 3, 3],
    },
    MonsterSpec {
        name: "The Drowned Court",
        health: 5810,
        strength: 146,
        regen: 13,
        mind_resist: 88,
        physical_resist: 64,
        magic_resist: 62,
        curse_resist: 88,
        attacks: &[],
        gear: &[
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 0, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 1, 0),
            ("Grasping Ring", SlotKind::Gloves, 5, 0, 0),
            ("Mage's Wrapping", SlotKind::Gloves, 4, 1, 0),
            ("Flaying Mold", SlotKind::Gloves, 2, 1, 3),
            ("Siphon Ring", SlotKind::Gloves, 1, 2, 0),
            ("Ring of Tides", SlotKind::Gloves, 0, 2, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 3, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 3, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 4, 0),
            ("Seal of the Deep", SlotKind::Gloves, 5, 3, 1),
            ("Rootwoven Material", SlotKind::Gloves, 0, 5, 0),
            ("Flaying Mold", SlotKind::Gloves, 3, 4, 2),
            ("Seal of the Deep", SlotKind::Gloves, 0, 6, 0),
            ("Grasping Ring", SlotKind::Gloves, 2, 4, 0),
            ("Rootwoven Material", SlotKind::Gloves, 2, 6, 0),
            ("Gauntlet Mold", SlotKind::Gloves, 3, 6, 3),
            ("Grasping Ring", SlotKind::Gloves, 5, 5, 0),
            ("Siphon Ring", SlotKind::Gloves, 2, 7, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Crown of the Deep", SlotKind::Helmet, 0, 2, 0),
            ("Overseer's Circlet", SlotKind::Helmet, 3, 2, 0),
            ("Consecrated Plating", SlotKind::Helmet, 2, 4, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 3, 3),
            ("Overseer's Circlet", SlotKind::Helmet, 4, 4, 1),
            ("Consecrated Plating", SlotKind::Helmet, 2, 6, 0),
            ("Mirrored Visor", SlotKind::Helmet, 0, 5, 1),
        ],
        gear_offset: 0,
        bounty: 361,
        sprite: MonsterSprite::Drowned,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[4, 4, 4, 4, 4, 3, 3, 3],
    },
    MonsterSpec {
        name: "Anvilheart",
        health: 6150,
        strength: 154,
        regen: 14,
        mind_resist: 90,
        physical_resist: 66,
        magic_resist: 64,
        curse_resist: 90,
        attacks: &[],
        gear: &[
            ("Stonewall Frame", SlotKind::Helmet, 0, 0, 0),
            ("Consecrated Plating", SlotKind::Helmet, 3, 0, 0),
            ("Godsteel Plating", SlotKind::Helmet, 0, 2, 0),
            ("Crown of the Deep", SlotKind::Helmet, 3, 2, 0),
            ("Bulwark Base", SlotKind::Chest, 0, 0, 0),
            ("Godsheet Layer", SlotKind::Chest, 0, 2, 0),
            ("Godsheet Layer", SlotKind::Chest, 3, 2, 0),
            ("Warlord's Pauldron", SlotKind::Chest, 4, 0, 0),
            ("Titan's Grip", SlotKind::Gloves, 0, 0, 0),
            ("Sovereign Mold", SlotKind::Gloves, 3, 0, 0),
            ("Seal of Power", SlotKind::Gloves, 0, 2, 0),
            ("Warding Ring", SlotKind::Gloves, 3, 1, 0),
            ("Worldweave Material", SlotKind::Greaves, 0, 0, 0),
            ("Godsteel Plating", SlotKind::Greaves, 3, 0, 0),
            ("Sevenleague Sole", SlotKind::Greaves, 3, 1, 0),
            ("Sunderer", SlotKind::Weapon, 0, 0, 0),
            ("Sunderer", SlotKind::Weapon, 2, 0, 0),
            ("Kingmaker Hilt", SlotKind::Weapon, 0, 3, 0),
            ("Grimoire Rack", SlotKind::Weapon, 4, 0, 0),
            ("Grimoire Rack", SlotKind::Weapon, 5, 0, 0),
            ("Ossuary Frame", SlotKind::Helmet, 0, 3, 0),
            ("Lonely Plating", SlotKind::Helmet, 2, 4, 0),
        ],
        gear_offset: 0,
        bounty: 372,
        sprite: MonsterSprite::Anvil,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[],
    },
    MonsterSpec {
        name: "The Salt Wedding",
        health: 6510,
        strength: 163,
        regen: 14,
        mind_resist: 92,
        physical_resist: 68,
        magic_resist: 66,
        curse_resist: 92,
        attacks: &[],
        gear: &[
            ("Stonewall Frame", SlotKind::Helmet, 0, 0, 0),
            ("Godsteel Plating", SlotKind::Helmet, 3, 0, 0),
            ("Godsteel Plating", SlotKind::Helmet, 0, 2, 0),
            ("Warlord's Crest", SlotKind::Helmet, 3, 1, 0),
            ("Bulwark Base", SlotKind::Chest, 0, 0, 0),
            ("Godsheet Layer", SlotKind::Chest, 0, 2, 0),
            ("Godsheet Layer", SlotKind::Chest, 3, 2, 0),
            ("Godsheet Layer", SlotKind::Chest, 0, 4, 0),
            ("Titan's Grip", SlotKind::Gloves, 0, 0, 0),
            ("Sovereign Mold", SlotKind::Gloves, 3, 0, 0),
            ("Seal of Power", SlotKind::Gloves, 0, 2, 0),
            ("Seal of the Deep", SlotKind::Gloves, 2, 2, 0),
            ("Sevenleague Boots", SlotKind::Greaves, 0, 0, 0),
            ("Godsteel Plating", SlotKind::Greaves, 2, 0, 0),
            ("Sevenleague Sole", SlotKind::Greaves, 5, 0, 0),
            ("Sunderer", SlotKind::Weapon, 0, 0, 0),
            ("Sunderer", SlotKind::Weapon, 2, 0, 0),
            ("Kingmaker Hilt", SlotKind::Weapon, 0, 3, 0),
            ("Grimoire Rack", SlotKind::Weapon, 4, 0, 0),
            ("Bileglass Vial", SlotKind::Weapon, 4, 2, 0),
        ],
        gear_offset: 0,
        bounty: 383,
        sprite: MonsterSprite::Wedding,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[],
    },
    MonsterSpec {
        name: "Nine of Ashes",
        health: 6890,
        strength: 172,
        regen: 15,
        mind_resist: 93,
        physical_resist: 70,
        magic_resist: 68,
        curse_resist: 93,
        attacks: &[],
        gear: &[
            ("Anvil Frame", SlotKind::Helmet, 0, 0, 0),
            ("Lonely Plating", SlotKind::Helmet, 3, 0, 0),
            ("Lonely Plating", SlotKind::Helmet, 3, 1, 0),
            ("Buttressed Frame", SlotKind::Helmet, 0, 2, 0),
            ("Lonely Plating", SlotKind::Helmet, 3, 2, 0),
            ("Lonely Plating", SlotKind::Helmet, 2, 3, 0),
            ("Coven Crest", SlotKind::Helmet, 5, 1, 0),
            ("Ossuary Frame", SlotKind::Helmet, 0, 3, 0),
            ("Scarred Plating", SlotKind::Helmet, 2, 4, 0),
            ("Scarred Plating", SlotKind::Helmet, 4, 4, 0),
            ("The Empty Crown", SlotKind::Helmet, 4, 3, 0),
            ("Cinder Base", SlotKind::Chest, 0, 0, 0),
            ("Warlord's Pauldron", SlotKind::Chest, 3, 0, 0),
            ("Warlord's Pauldron", SlotKind::Chest, 2, 1, 0),
            ("Ungloved Layer", SlotKind::Chest, 0, 2, 0),
            ("Grove Base", SlotKind::Chest, 1, 3, 0),
            ("Riveted Layer", SlotKind::Chest, 3, 4, 0),
            ("The Growing Weight", SlotKind::Chest, 4, 2, 0),
            ("Grove Base", SlotKind::Chest, 0, 5, 0),
            ("Wildfire Layer", SlotKind::Chest, 3, 5, 0),
            ("The Growing Weight", SlotKind::Chest, 2, 6, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Vicegrip Mold", SlotKind::Gloves, 3, 0, 0),
            ("Deepdraught Ring", SlotKind::Gloves, 5, 0, 0),
            ("Unshod Signet", SlotKind::Gloves, 0, 1, 0),
            ("Rootwoven Material", SlotKind::Gloves, 1, 1, 0),
            ("Vicegrip Mold", SlotKind::Gloves, 4, 1, 0),
            ("Unshod Signet", SlotKind::Gloves, 1, 2, 0),
            ("Rootwoven Material", SlotKind::Gloves, 2, 2, 0),
            ("Vicegrip Mold", SlotKind::Gloves, 1, 3, 0),
            ("Ring of Embers", SlotKind::Gloves, 5, 2, 0),
            ("Unshod Signet", SlotKind::Gloves, 0, 3, 0),
            ("Ashwoven Material", SlotKind::Greaves, 0, 0, 0),
            ("Deeprooted Sole", SlotKind::Greaves, 3, 0, 0),
            ("Overflow Plate", SlotKind::Greaves, 0, 1, 0),
            ("Reliquary Sole", SlotKind::Greaves, 2, 1, 0),
            ("Widow's Sole", SlotKind::Greaves, 4, 1, 0),
            ("Warded Plating", SlotKind::Greaves, 1, 3, 0),
            ("Sevenleague Boots", SlotKind::Greaves, 3, 3, 0),
            ("Widow's Sole", SlotKind::Greaves, 5, 2, 0),
            ("Mana Ward", SlotKind::Greaves, 0, 5, 0),
            ("Zealot's Haft", SlotKind::Weapon, 0, 0, 0),
            ("Gluttonous Fang", SlotKind::Weapon, 1, 0, 0),
            ("Fury Sigil", SlotKind::Weapon, 2, 0, 0),
            ("Bloodstone Bead", SlotKind::Weapon, 3, 0, 0),
            ("Zealot's Haft", SlotKind::Weapon, 5, 0, 0),
            ("Gluttonous Fang", SlotKind::Weapon, 3, 1, 0),
            ("Grudge Bead", SlotKind::Weapon, 4, 1, 0),
            ("Zealot's Haft", SlotKind::Weapon, 1, 2, 0),
            ("Gluttonous Fang", SlotKind::Weapon, 2, 2, 0),
            ("Oathstone Bead", SlotKind::Weapon, 4, 3, 0),
        ],
        gear_offset: 0,
        bounty: 394,
        sprite: MonsterSprite::Ashes,
        rank: Rank::Boss,
        drops: &["Tetrahedron Shard"],
        items: &[3, 4, 4, 4, 3, 3, 4, 3, 4, 3, 3, 3, 4, 3, 3],
    },
    MonsterSpec {
        name: "The Last Light",
        health: 7290,
        strength: 182,
        regen: 16,
        mind_resist: 94,
        physical_resist: 72,
        magic_resist: 70,
        curse_resist: 94,
        attacks: &[],
        gear: &[
            ("Stonewall Frame", SlotKind::Helmet, 0, 0, 0),
            ("Godsteel Plating", SlotKind::Helmet, 3, 0, 0),
            ("Godsteel Plating", SlotKind::Helmet, 0, 2, 0),
            ("Martyr's Crest", SlotKind::Helmet, 3, 2, 0),
            ("Adamant Carapace", SlotKind::Chest, 0, 0, 0),
            ("Godsheet Layer", SlotKind::Chest, 0, 3, 0),
            ("Godsheet Layer", SlotKind::Chest, 3, 3, 0),
            ("Warlord's Pauldron", SlotKind::Chest, 4, 0, 0),
            ("Titan's Grip", SlotKind::Gloves, 0, 0, 0),
            ("Sovereign Mold", SlotKind::Gloves, 3, 0, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 2, 0),
            ("Warding Ring", SlotKind::Gloves, 3, 1, 0),
            ("Titan's Grip", SlotKind::Greaves, 0, 0, 0),
            ("Godsteel Plating", SlotKind::Greaves, 3, 0, 0),
            ("Sevenleague Sole", SlotKind::Greaves, 3, 1, 0),
            ("Sunderer", SlotKind::Weapon, 0, 0, 0),
            ("Sunderer", SlotKind::Weapon, 2, 0, 0),
            ("Kingmaker Hilt", SlotKind::Weapon, 0, 3, 0),
            ("Bileglass Vial", SlotKind::Weapon, 4, 0, 0),
            ("Duelist's Fob", SlotKind::Weapon, 4, 1, 0),
        ],
        gear_offset: 0,
        bounty: 405,
        sprite: MonsterSprite::Lantern,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[],
    },
    MonsterSpec {
        name: "Gilt",
        health: 7720,
        strength: 192,
        regen: 17,
        mind_resist: 95,
        physical_resist: 74,
        magic_resist: 72,
        curse_resist: 95,
        attacks: &[],
        gear: &[
            ("Aegis Crown", SlotKind::Helmet, 0, 0, 0),
            ("Lonely Plating", SlotKind::Helmet, 3, 0, 0),
            ("Heartwood Crest", SlotKind::Helmet, 3, 1, 0),
            ("Aegis Crown", SlotKind::Helmet, 3, 2, 0),
            ("Lonely Plating", SlotKind::Helmet, 1, 3, 0),
            ("Lonely Plating", SlotKind::Helmet, 0, 4, 0),
            ("Bastion Base", SlotKind::Chest, 0, 0, 0),
            ("Warlord's Pauldron", SlotKind::Chest, 3, 0, 0),
            ("The Growing Weight", SlotKind::Chest, 0, 2, 0),
            ("Grove Base", SlotKind::Chest, 2, 2, 0),
            ("Warlord's Pauldron", SlotKind::Chest, 4, 3, 0),
            ("Warlord's Pauldron", SlotKind::Chest, 2, 4, 0),
            ("Ungloved Layer", SlotKind::Chest, 0, 4, 0),
            ("Rootwoven Material", SlotKind::Gloves, 0, 0, 0),
            ("Vicegrip Mold", SlotKind::Gloves, 3, 0, 0),
            ("Grasping Ring", SlotKind::Gloves, 5, 0, 0),
            ("Unshod Signet", SlotKind::Gloves, 0, 1, 0),
            ("Rootwoven Material", SlotKind::Gloves, 1, 1, 0),
            ("Wrathful Mold", SlotKind::Gloves, 0, 2, 0),
            ("Deepdraught Ring", SlotKind::Gloves, 4, 1, 0),
            ("Unshod Signet", SlotKind::Gloves, 5, 1, 0),
            ("Worldweave Material", SlotKind::Greaves, 0, 0, 0),
            ("Deeprooted Sole", SlotKind::Greaves, 3, 0, 0),
            ("Warded Plating", SlotKind::Greaves, 3, 1, 0),
            ("Sevenleague Boots", SlotKind::Greaves, 0, 2, 0),
            ("Sevenleague Sole", SlotKind::Greaves, 2, 2, 0),
            ("Lonely Plating", SlotKind::Greaves, 3, 3, 0),
            ("Zealot's Haft", SlotKind::Weapon, 0, 0, 0),
            ("Gluttonous Fang", SlotKind::Weapon, 1, 0, 0),
            ("Fury Sigil", SlotKind::Weapon, 2, 0, 0),
            ("Fury Sigil", SlotKind::Weapon, 3, 0, 0),
            ("Zealot's Haft", SlotKind::Weapon, 4, 0, 0),
            ("Gluttonous Fang", SlotKind::Weapon, 3, 2, 0),
            ("Grudge Bead", SlotKind::Weapon, 5, 0, 0),
            ("Bloodstone Bead", SlotKind::Weapon, 1, 2, 0),
        ],
        gear_offset: 0,
        bounty: 416,
        sprite: MonsterSprite::Gilt,
        rank: Rank::Mini,
        drops: &["Gilded Offcuts"],
        items: &[3, 3, 3, 4, 4, 4, 3, 3, 4, 4],
    },
    // The top of the ladder. Everything above the Gearwright wears the best
    // the shop can sell; Francis wears something it never could.
    MonsterSpec {
        name: "Francis",
        health: 9400,
        strength: 215,
        regen: 22,
        mind_resist: 96,
        physical_resist: 78,
        magic_resist: 76,
        curse_resist: 96,
        attacks: &[],
        // Ninety-five percent of his cells, in nineteen items. He was on
        // thirty-six percent with one item a slot, which is not a hard fight,
        // it is four fifths of an empty board - the two finished human boards
        // in `share` pack ninety-seven and ninety-eight. Laid out by
        // `tests/pack_francis.rs` rather than by hand.
        gear: &[
            ("Buttressed Frame", SlotKind::Helmet, 0, 0, 0),
            ("Deadweight Plating", SlotKind::Helmet, 3, 0, 1),
            ("Warded Plating", SlotKind::Helmet, 2, 1, 0),
            ("Buttressed Frame", SlotKind::Helmet, 0, 1, 3),
            ("Broken Crown", SlotKind::Helmet, 1, 3, 0),
            ("Warded Plating", SlotKind::Helmet, 4, 1, 0),
            ("Zealot's Crest", SlotKind::Helmet, 0, 4, 1),
            ("Buttressed Frame", SlotKind::Helmet, 2, 5, 0),
            ("Deadweight Plating", SlotKind::Helmet, 0, 6, 1),
            ("Warded Plating", SlotKind::Helmet, 4, 6, 0),
            ("Zealot's Crest", SlotKind::Helmet, 0, 7, 0),
            ("The Money Jacket", SlotKind::Chest, 0, 0, 0),
            ("Runic Weave", SlotKind::Chest, 4, 0, 1),
            ("Bastion Base", SlotKind::Chest, 0, 3, 0),
            ("Bulwark Layer", SlotKind::Chest, 3, 3, 0),
            ("Verdant Weave", SlotKind::Chest, 5, 1, 1),
            ("Verdant Weave", SlotKind::Chest, 0, 5, 0),
            ("Bastion Base", SlotKind::Chest, 3, 5, 0),
            ("Runic Weave", SlotKind::Chest, 0, 6, 0),
            ("Breaker's Fist", SlotKind::Gloves, 0, 0, 1),
            ("Vicegrip Mold", SlotKind::Gloves, 3, 0, 0),
            ("Breaker's Fist", SlotKind::Gloves, 3, 1, 1),
            ("Vicegrip Mold", SlotKind::Gloves, 1, 2, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 2, 1),
            ("Warding Ring", SlotKind::Gloves, 5, 0, 0),
            ("Breaker's Fist", SlotKind::Gloves, 1, 3, 1),
            ("Vicegrip Mold", SlotKind::Gloves, 4, 3, 0),
            ("Breaker's Fist", SlotKind::Gloves, 4, 4, 0),
            ("Vicegrip Mold", SlotKind::Gloves, 2, 5, 0),
            ("Seal of the Deep", SlotKind::Gloves, 0, 5, 0),
            ("Warding Ring", SlotKind::Gloves, 0, 4, 0),
            ("Breaker's Fist", SlotKind::Gloves, 0, 6, 1),
            ("Vicegrip Mold", SlotKind::Gloves, 3, 6, 1),
            ("Breaker's Fist", SlotKind::Greaves, 0, 0, 1),
            ("Witherroot", SlotKind::Greaves, 3, 0, 3),
            ("Deadweight Plating", SlotKind::Greaves, 5, 0, 0),
            ("Tallykeeper's Weave", SlotKind::Greaves, 0, 2, 0),
            ("Witherroot", SlotKind::Greaves, 2, 2, 3),
            ("Broken Crown", SlotKind::Greaves, 0, 4, 0),
            ("Breaker's Fist", SlotKind::Greaves, 1, 6, 1),
            ("Witherroot", SlotKind::Greaves, 4, 6, 1),
            ("Deadweight Plating", SlotKind::Greaves, 5, 3, 0),
            ("Ironbound Haft", SlotKind::Weapon, 0, 0, 1),
            ("Bonesaw", SlotKind::Weapon, 3, 0, 0),
        ],
        gear_offset: 0,
        bounty: 500,
        sprite: MonsterSprite::Francis,
        rank: Rank::Ordinary,
        drops: &[],
        items: &[3, 4, 4, 2, 4, 2, 2, 4, 2, 4, 2, 3, 3, 3, 2],
    },
];

// ----------------------------------------------------------- combatants

/// An item mid-fight: its profile plus how far its cooldown has filled.
///
/// `Default` is here for tests that care about one field - which item a stun
/// picks depends on `rating` and `stun_ms` and nothing else, and spelling out
/// thirty irrelevant fields to say so buries the point.
#[derive(Clone, Debug, Default)]
pub struct RunningItem {
    pub name: String,
    /// Effectiveness on the shared scale, so the interface can badge it.
    pub rating: i32,
    pub slot: Option<SlotKind>,
    pub cooldown_ms: u32,
    pub progress_ms: u32,
    /// How much longer this one item is stopped for. A stun holds a single
    /// item rather than the whole fighter, so it lives here.
    pub stun_ms: u32,
    /// Run gold this item has spent so far this fight, and how many times it
    /// has paid. The budget belongs to the item, so the tally does too.
    pub gold_spent: i32,
    pub gold_paid: u32,
    /// What this item multiplies its own damage by, in hundredths.
    pub power: i32,
    pub physical_damage: i32,
    pub magic_damage: i32,
    pub mind: i32,
    pub armor: i32,
    pub mana: i32,
    pub rage: i32,
    pub faith: i32,
    pub nature: i32,
    pub triggers: Vec<Trigger>,
    pub adjacent_assembled_same_slot: usize,
    /// Empty cells touching this item on the board it was built on.
    pub open_cells: usize,
    /// Indices, in the owner's item list, of items this one reacts to.
    pub adjacent_items: Vec<usize>,
    pub aligned_items: Vec<usize>,
    pub diagonal_items: Vec<usize>,
    /// One tally per entry in `triggers`, so a `Watch` remembers what it has
    /// seen. Parallel to `triggers` rather than keyed by anything, because two
    /// identical watchers on one item are two separate counts.
    pub watched: Vec<u32>,
    /// Which watchers have already paid out. Only read by the ones that do not
    /// repeat.
    pub watch_paid: Vec<bool>,
    /// Monster attacks can carry a curse; player items use triggers instead.
    pub curse: Option<CurseKind>,
    /// Fingerprint used to draw this item's emblem.
    pub sigil_seed: u64,
    /// Weapon power that applies to this item alone - a spell's ink.
    pub power_bonus: i32,
    /// The payloads a spell cycles through. Empty for ordinary gear.
    pub casts: Vec<crate::loadout::Cast>,
    /// Which payload the next cast will use.
    pub cast_index: usize,
}

impl RunningItem {
    fn from_profile(p: &ItemProfile) -> Self {
        RunningItem {
            name: p.name.clone(),
            slot: Some(p.slot),
            cooldown_ms: p.cooldown_ms,
            progress_ms: 0,
            stun_ms: 0,
            gold_spent: 0,
            gold_paid: 0,
            physical_damage: p.stats.physical_damage,
            magic_damage: p.stats.magic_damage,
            rage: p.stats.rage,
            faith: p.stats.faith,
            nature: p.stats.nature,
            mind: p.stats.mind,
            armor: p.stats.armor,
            mana: p.stats.mana,
            triggers: p.triggers.clone(),
            adjacent_assembled_same_slot: p.adjacent_assembled_same_slot,
            open_cells: p.open_cells,
            power: p.power,
            adjacent_items: p.adjacent_items.clone(),
            aligned_items: p.aligned_items.clone(),
            diagonal_items: p.diagonal_items.clone(),
            watched: vec![0; p.triggers.len()],
            watch_paid: vec![false; p.triggers.len()],
            curse: None,
            sigil_seed: p.sigil_seed,
            rating: p.rating,
            power_bonus: p.power_bonus,
            casts: p.casts.clone(),
            cast_index: 0,
        }
    }

    fn from_attack(a: &MonsterAttack) -> Self {
        RunningItem {
            name: a.name.to_string(),
            slot: None,
            cooldown_ms: a.cooldown_ms.max(TICK_MS),
            progress_ms: 0,
            stun_ms: 0,
            gold_spent: 0,
            gold_paid: 0,
            physical_damage: a.damage,
            magic_damage: 0,
            rage: 0,
            faith: 0,
            nature: 0,
            mind: a.mind,
            armor: a.armor,
            mana: 0,
            triggers: Vec::new(),
            adjacent_assembled_same_slot: 0,
            open_cells: 0,
            power: 100,
            adjacent_items: Vec::new(),
            aligned_items: Vec::new(),
            diagonal_items: Vec::new(),
            watched: Vec::new(),
            watch_paid: Vec::new(),
            curse: a.curse,
            // Innate attacks have no gear behind them, so seed off the name.
            rating: 0,
            power_bonus: 0,
            casts: Vec::new(),
            cast_index: 0,
            sigil_seed: a.name.bytes().fold(0x1234_5678_u64, |h, b| {
                h.rotate_left(5) ^ b as u64
            }),
        }
    }

    /// Fraction of the way to the next activation, for cooldown bars.
    pub fn progress(&self) -> f32 {
        if self.cooldown_ms == 0 {
            return 0.0;
        }
        (self.progress_ms as f32 / self.cooldown_ms as f32).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug)]
pub struct Combatant {
    pub name: String,
    pub max_health: i32,
    pub health: i32,
    /// Temporary hit points. Always starts a fight at zero — gear has to build
    /// it up — and soaks damage before health does.
    pub armor: i32,
    pub mana: i32,
    pub strength: i32,
    pub power: i32,
    pub regen: i32,
    pub mind_resist: i32,
    pub curse_resist: i32,
    // The defence triangle, per damage type. See `stats::after_defences`.
    pub physical_resist: i32,
    pub physical_pierce: i32,
    pub physical_harden: i32,
    pub magic_resist: i32,
    pub magic_pierce: i32,
    pub magic_harden: i32,
    /// Percent of absorbed damage turned back on whoever swung.
    pub reflect: i32,
    /// Banked resources. Each is spent by triggers and worth something merely
    /// by being held - see `held_bonus`.
    pub rage: i32,
    pub faith: i32,
    pub nature: i32,
    /// The fused pools. Made by `Action::Fuse`, worth both parents at double
    /// rate, and spendable by nothing - the only way one leaves is a `Drain`.
    pub druidic_might: i32,
    pub communion: i32,
    pub zealotry: i32,
    /// Run gold carried into the fight. Only the player has one, and only
    /// `SpendGold` touches it; what it spends is gone at the shop afterwards.
    pub purse: i32,
    /// Which foe the next single-target attack is aimed at. Only the player
    /// has one, and it moves along every time an attack lands - see `aim_of`.
    pub aim: usize,
    /// Set by Immense Guilt: regeneration does nothing at all.
    pub no_regen: bool,
    /// Set by Trundle: everything runs this much slower, and every point of
    /// armour counts this much. Percentages; 0 and 100 mean "as written".
    pub slower_pct: i32,
    pub armour_pct: i32,
    /// Set by Longhauler: everything runs this much faster for every second
    /// the fight has been going, capped at twice speed.
    pub haste_per_s: i32,
    /// Set by Ticket to Ride: every `n`th attack made against this fighter
    /// misses entirely. Zero means nothing misses.
    pub warded_every: u32,
    /// Attacks this fighter has made that were counted against a ward. Kept on
    /// the attacker, so two creatures each miss every other swing rather than
    /// sharing one tally between them.
    pub warded_count: u32,
    pub curses: Curses,
    /// Stacks of mana empowerment and mana shield. Both scale off *current*
    /// mana, and both are bought with mana — so stacking them hard drains the
    /// very pool they multiply. That tension is the point.
    pub empowerment: u32,
    pub shield: u32,
    /// Stacks of spell forking: every cast lands once more per stack.
    pub forking: u32,
    pub items: Vec<RunningItem>,
    /// Sub-point accumulators, so 10 damage a second spread over 50ms ticks
    /// loses nothing to rounding.
    /// Chronomancer's slow time: damage waiting to arrive, and how long each
    /// portion has left. Empty for everyone else.
    pending: Vec<(i32, u32)>,
    /// Whether incoming damage is queued rather than taken at once.
    /// Seconds damage is spread over. Zero means it lands at once.
    pub slow_time: u32,
    /// Held resources count double.
    /// How many times a held pool counts. One is ordinary.
    pub overflowing: i32,
    /// Percent of damage dealt that comes back as health.
    pub leech: i32,
    /// Every nth activation fires twice. Zero means never.
    pub echo_every: u32,
    /// Percent of absorbed damage handed back as armour.
    pub bastion: i32,
    /// Curses landed bring the other kind with them.
    /// Extra curses dragged in alongside each one landed.
    pub contagion: u32,
    /// Faith banked whenever a hit lands on you.
    pub reprisal: i32,
    /// Milliseconds every enemy activation gives back to your cooldowns.
    pub riposte: u32,
    /// Strength gained per second the fight has run.
    pub momentum: i32,
    /// Reactions fire twice.
    /// How many times a reaction pays out. One is ordinary.
    pub resonance: u32,
    /// Percent of physical damage that lands again as magic.
    pub transmute: i32,
    /// Every activation banks one of each pool.
    /// Of each pool banked per activation. Zero is ordinary.
    pub adaptable: i32,
    /// Oracle: every this-many-th activation lands the two curses that work on
    /// time - a stun and a misfire.
    pub untimely: u32,
    /// Stormcaller: every activation pushes every OTHER item's cooldown
    /// forward by this many ms, so a fast build compounds on itself.
    pub cascade: u32,
    /// Warpriest: armour gained is this much stronger, in percent, while any
    /// faith is held.
    pub consecrate: i32,
    /// Activations counted for the misfire curse. Counting rather than rolling
    /// keeps the fight deterministic.
    pub misfire_count: u32,
    /// How many stuns this fighter has taken. Mixed into the choice of which
    /// item the next one lands on, so a chain of stuns walks across the kit
    /// instead of hammering one slot.
    pub stun_count: u32,
    /// The same, for an Oracle's periodic reach at the clock.
    pub untimely_count: u32,
    /// Bloodletter: landing a curse banks this much rage.
    pub bloodscent: i32,
    /// Wellspring: spending a pool refunds this percent of it to each of the
    /// other three.
    pub confluence: i32,
    /// How many times this side has activated anything, for `echo_every`.
    activations: u32,
    dot_milli: i32,
    regen_milli: i32,
    /// Burn damage already taken but not yet written to the log, and how long
    /// since the last entry. Damage-over-time lands every tick; logging it
    /// every tick buries everything else under a wall of "burns for 1".
    burn_acc: i32,
    burn_timer: u32,
}

impl Combatant {
    pub fn player(stats: Stats, profiles: &[ItemProfile]) -> Self {
        Combatant {
            name: "You".to_string(),
            max_health: stats.health,
            health: stats.health,
            armor: 0,
            mana: 0,
            druidic_might: 0,
            communion: 0,
            zealotry: 0,
            strength: stats.strength,
            power: stats.power,
            regen: stats.regen,
            mind_resist: stats.mind_resist,
            physical_resist: stats.physical_resist,
            physical_pierce: stats.physical_pierce,
            physical_harden: stats.physical_harden,
            magic_resist: stats.magic_resist,
            magic_pierce: stats.magic_pierce,
            magic_harden: stats.magic_harden,
            reflect: stats.reflect,
            rage: 0,
            faith: 0,
            nature: 0,
            pending: Vec::new(),
            slow_time: 0,
            overflowing: 1,
            leech: 0,
            echo_every: 0,
            bastion: 0,
            contagion: 0,
            reprisal: 0,
            riposte: 0,
            momentum: 0,
            resonance: 1,
            transmute: 0,
            adaptable: 0,
            untimely: 0,
            cascade: 0,
            consecrate: 0,
            misfire_count: 0,
            stun_count: 0,
            untimely_count: 0,
            bloodscent: 0,
            confluence: 0,
            activations: 0,
            curse_resist: stats.curse_resist,
            purse: 0,
            aim: 0,
            no_regen: false,
            slower_pct: 0,
            armour_pct: 100,
            haste_per_s: 0,
            warded_every: 0,
            warded_count: 0,
            curses: Curses::new(),
            empowerment: 0,
            shield: 0,
            forking: 0,
            items: profiles.iter().map(RunningItem::from_profile).collect(),
            dot_milli: 0,
            regen_milli: 0,
            burn_acc: 0,
            burn_timer: 0,
        }
    }

    pub fn monster(spec: &MonsterSpec) -> Self {
        Combatant::monster_at(spec, Difficulty::Easy)
    }

    pub fn monster_at(spec: &MonsterSpec, difficulty: Difficulty) -> Self {
        // Most of the setting is in what it is wearing; the multiplier below
        // is only what is left over.
        let (mut stats, profiles) = spec.outfit_at(difficulty);

        // Half the difficulty goes into staying alive and half into hitting
        // back, so the two multiply out to the factor on the tin.
        let each = difficulty.each_way();
        stats.health = ((stats.health as f32) * each).round() as i32;
        stats.strength = ((stats.strength as f32) * each).round() as i32;

        let mut haste = 100;
        for passive in difficulty.passives() {
            match passive {
                Passive::Hardened => stats.regen += 4,
                Passive::Warded => {
                    stats.mind_resist += 40;
                    stats.curse_resist += 40;
                    stats.physical_resist += 20;
                    stats.magic_resist += 20;
                }
                Passive::Relentless => haste = 125,
            }
        }
        // Innate attacks first, then anything its gear assembles.
        let mut items: Vec<RunningItem> =
            spec.attacks.iter().map(RunningItem::from_attack).collect();
        items.extend(profiles.iter().map(RunningItem::from_profile));
        if haste != 100 {
            for it in &mut items {
                it.cooldown_ms = ((it.cooldown_ms as i64 * 100 / haste as i64) as u32).max(TICK_MS);
            }
        }
        Combatant {
            name: spec.name.to_string(),
            max_health: stats.health,
            health: stats.health,
            armor: 0,
            mana: 0,
            druidic_might: 0,
            communion: 0,
            zealotry: 0,
            strength: stats.strength,
            power: stats.power,
            regen: stats.regen,
            mind_resist: stats.mind_resist,
            physical_resist: stats.physical_resist,
            physical_pierce: stats.physical_pierce,
            physical_harden: stats.physical_harden,
            magic_resist: stats.magic_resist,
            magic_pierce: stats.magic_pierce,
            magic_harden: stats.magic_harden,
            reflect: stats.reflect,
            rage: 0,
            faith: 0,
            nature: 0,
            pending: Vec::new(),
            slow_time: 0,
            overflowing: 1,
            leech: 0,
            echo_every: 0,
            bastion: 0,
            contagion: 0,
            reprisal: 0,
            riposte: 0,
            momentum: 0,
            resonance: 1,
            transmute: 0,
            adaptable: 0,
            untimely: 0,
            cascade: 0,
            consecrate: 0,
            misfire_count: 0,
            stun_count: 0,
            untimely_count: 0,
            bloodscent: 0,
            confluence: 0,
            activations: 0,
            curse_resist: stats.curse_resist,
            purse: 0,
            aim: 0,
            no_regen: false,
            slower_pct: 0,
            armour_pct: 100,
            haste_per_s: 0,
            warded_every: 0,
            warded_count: 0,
            curses: Curses::new(),
            empowerment: 0,
            shield: 0,
            forking: 0,
            items,
            dot_milli: 0,
            regen_milli: 0,
            burn_acc: 0,
            burn_timer: 0,
        }
    }

    pub fn is_down(&self) -> bool {
        self.health <= 0 || self.max_health <= 0
    }

    /// Weapon power after mana empowerment: 0.05x per stack per point of mana.
    /// What the resources you are holding are worth right now. Spending them
    /// gives it up, which is the whole tension: a hoarded pool is a standing
    /// bonus, and a spent one is a burst.
    /// One of the four banked pools, by name.
    /// Take on armour, counting whatever Trundle makes a point worth.
    ///
    /// Every route to armour goes through here. It used to be four separate
    /// `armor +=` sites, which is three chances to add a multiplier and one
    /// chance to forget.
    pub fn gain_armor(&mut self, n: i32) -> i32 {
        let got = (n as i64 * self.armour_pct as i64 / 100) as i32;
        self.armor += got;
        got
    }

    pub fn pool(&self, what: crate::piece::Resource) -> i32 {
        use crate::piece::Resource::*;
        match what {
            Mana => self.mana,
            Rage => self.rage,
            Faith => self.faith,
            Nature => self.nature,
            DruidicMight => self.druidic_might,
            Communion => self.communion,
            Zealotry => self.zealotry,
        }
    }

    pub fn set_pool(&mut self, what: crate::piece::Resource, v: i32) {
        use crate::piece::Resource::*;
        match what {
            Mana => self.mana = v,
            Rage => self.rage = v,
            Faith => self.faith = v,
            Nature => self.nature = v,
            DruidicMight => self.druidic_might = v,
            Communion => self.communion = v,
            Zealotry => self.zealotry = v,
        }
    }

    pub fn held_bonus(&self) -> Stats {
        let m = self.overflowing.max(1);
        let (rage, faith, nature) = (self.rage * m, self.faith * m, self.nature * m);
        // A fusion pays both its parents, each at double the parent's rate.
        // Written out rather than derived from `parents()` because the rates
        // are the design and reading them off a table hides what they are.
        let (might, comm, zeal) =
            (self.druidic_might * m, self.communion * m, self.zealotry * m);
        Stats {
            // Fury sharpens the blade.
            physical_damage: rage + might * 2 + zeal * 2,
            // Conviction turns aside both kinds of harm, and no longer stops
            // at forty percent. The cap meant a faith build hit a ceiling it
            // could not see and everything banked past it was dead weight -
            // which is the opposite of what a pool is for.
            physical_resist: faith * 2 + comm * 4 + zeal * 4,
            magic_resist: faith * 2 + comm * 4 + zeal * 4,
            // Growth knits you back together.
            regen: nature + might * 2 + comm * 2,
            ..Stats::ZERO
        }
    }

    /// Regeneration a second, pools included.
    ///
    /// `held_bonus` has always computed this correctly and nothing has ever
    /// read it. The one call site takes `.physical_damage` and throws the rest
    /// away, so rage reached a fight and **nature and faith did not**: the
    /// regen tick read the flat `regen` field, `take_typed` read the flat
    /// resists, and a hundred banked nature was worth exactly nothing. The
    /// unit test that was supposed to cover it asserts `held_bonus()` directly
    /// and so has been green throughout, testing arithmetic nobody consulted.
    ///
    /// Named `effective_*`, beside `effective_power`, because that is the shape
    /// this file already uses for "the number after everything the wearer
    /// brings".
    pub fn effective_regen(&self) -> i32 {
        self.regen + self.held_bonus().regen
    }

    pub fn effective_physical_resist(&self) -> i32 {
        self.physical_resist + self.held_bonus().physical_resist
    }

    pub fn effective_magic_resist(&self) -> i32 {
        self.magic_resist + self.held_bonus().magic_resist
    }

    pub fn effective_power(&self) -> i32 {
        self.power + self.empowerment as i32 * 5 * self.mana.max(0)
    }

    /// Flat reduction mana shield applies to any incoming damage.
    pub fn damage_reduction(&self) -> i32 {
        self.shield as i32 * self.mana.max(0)
    }

    /// Mana shield first, then armour, then health. Returns (absorbed by
    /// armour, through to health).
    /// Take `amount` of `kind`, from an attacker with `pierce` percent
    /// piercing of that type.
    fn take_typed(&mut self, amount: i32, kind: DamageType, pierce: i32) -> (i32, i32) {
        let amount = match kind {
            DamageType::Physical => crate::stats::after_defences(
                amount,
                self.effective_physical_resist(),
                pierce,
                self.physical_harden,
            ),
            DamageType::Magic => crate::stats::after_defences(
                amount,
                self.effective_magic_resist(),
                pierce,
                self.magic_harden,
            ),
        };
        let amount = (amount - self.damage_reduction()).max(0);
        if amount <= 0 {
            return (0, 0);
        }
        if self.slow_time > 0 {
            // Nothing lands now. It arrives in slices over the next few
            // seconds, which is time for armour and regeneration to answer.
            self.pending.push((amount, self.slow_time * 1000));
            return (0, 0);
        }
        let absorbed = amount.min(self.armor.max(0));
        self.armor -= absorbed;
        let through = amount - absorbed;
        self.health -= through;
        // A wall that rebuilds itself under fire.
        if self.bastion > 0 && absorbed > 0 {
            self.gain_armor(absorbed * self.bastion / 100);
        }
        // Being ground down is itself a resource.
        if self.reprisal > 0 {
            self.faith += self.reprisal;
        }
        (absorbed, through)
    }

    /// Mind damage eats maximum health, so it can never be healed back off.
    fn take_mind(&mut self, raw: i32) -> i32 {
        // "whatever the damage type" — mana shield blunts mind damage too.
        let raw = (raw - self.damage_reduction()).max(0);
        let dealt = mind_damage_after_resist(raw, self.mind_resist);
        if dealt <= 0 {
            return 0;
        }
        self.max_health = (self.max_health - dealt).max(0);
        if self.health > self.max_health {
            self.health = self.max_health;
        }
        dealt
    }
}

// ----------------------------------------------------------------- log

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// An item finished its cooldown. Always precedes that item's effects.
    /// `index` is the item's position in its owner's list, so two items with
    /// the same name stay distinguishable.
    Activate { side: Side, item: String, index: usize },
    /// Rage, faith or nature banked.
    GainResource { side: Side, what: &'static str, amount: i32, total: i32 },
    /// A pool taken off someone. `amount` is what was actually there to take.
    Drained { on: Side, what: &'static str, amount: i32, total: i32 },
    /// Run gold spent mid-fight. `remaining` is what is left in the purse,
    /// which is what you will arrive at the shop with.
    Spent { side: Side, amount: i32, remaining: i32 },
    /// The fight has gone on long enough and is now ending itself. `pct` is
    /// the share of maximum health both sides are losing this second.
    SuddenDeath { pct: i32 },
    Hit { by: Side, damage: i32, absorbed: i32, target_health: i32, target_armor: i32 },
    /// An item came round and nothing happened - a misfire ate it.
    Misfired { side: Side, item: String },
    /// An attack was warded off before it landed. Ticket to Ride.
    Warded { side: Side, item: String },
    /// A spell went off. `paid` says whether it was cast in full or weakly.
    Cast { side: Side, paid: bool, cost: i32, remaining: i32 },
    /// Maximum health grew mid-fight.
    Grew { side: Side, amount: i32, total: i32 },
    MindHit { by: Side, amount: i32, target_max_health: i32 },
    GainArmor { side: Side, amount: i32, total: i32 },
    GainMana { side: Side, amount: i32, total: i32 },
    /// `paid` says which branch of a mana trigger ran.
    ManaCheck { side: Side, cost: i32, paid: bool, remaining: i32 },
    /// A spend against rage, faith or nature.
    ResourceCheck { side: Side, what: &'static str, cost: i32, paid: bool, remaining: i32 },
    /// `stacks` is the count *after* this one landed, so the interface can
    /// say "curse of searing x3" without keeping its own tally.
    Cursed { on: Side, kind: CurseKind, duration_ms: u32, stacks: u32 },
    /// A stun stopped one item. Its own event rather than a `Cursed`, because
    /// a stun rides on an item and the interface needs to know which one:
    /// `index` is that item's position in its owner's list, and `duration_ms`
    /// is the whole time it is now stopped for.
    Stunned { on: Side, index: usize, item: String, duration_ms: u32, aimed: bool },
    /// Damage-over-time landing this tick.
    Burn { side: Side, damage: i32, health: i32 },
    Regen { side: Side, amount: i32, health: i32 },
    /// A reaction pushed an item's cooldown forward.
    Hastened { side: Side, item: String, by_ms: u32 },
    /// Armour turned a blow back on whoever threw it.
    Reflected { side: Side, damage: i32 },
    /// Two pools became one of a fused pool. `total` is what is now held of
    /// it, and `from`/`and` are the parents with what each has left.
    ///
    /// The parents are named here rather than left implicit because two pools
    /// going down with no line to explain them is exactly the sort of thing a
    /// player notices and cannot account for. A fusion is the only action in
    /// the game that spends something it was not asked for.
    Fused {
        side: Side,
        what: &'static str,
        total: i32,
        from: (&'static str, i32),
        and: (&'static str, i32),
    },
    /// A watcher reached its count and paid out. The payload logs itself; this
    /// says which item was counting, so a log reads as cause then effect.
    Watched { side: Side, item: String, what: &'static str },
    /// A mana buff gained stacks. `total` is the new stack count.
    Empowered { side: Side, total: u32, power_bonus: i32 },
    Shielded { side: Side, total: u32, reduction: i32 },
    /// Spell forking gained. Every cast lands once more per stack.
    Forking { side: Side, total: u32 },
    Fell { side: Side },
    End { outcome: Outcome },
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub at_ms: u32,
    pub event: Event,
    /// Which foe this entry is about, when the fight has more than one of
    /// them. Zero in a duel, and zero for anything the player did to himself.
    ///
    /// One field here rather than a `who` on each of the twenty-odd `Event`
    /// variants that name a side. It is unambiguous because the player is
    /// always singular: when a foe acts this is the actor, when the player
    /// acts on a foe this is the victim, and there is never a third party.
    pub who: u8,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Outcome {
    Victory,
    Defeat,
    Stalemate,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Outcome::Victory => "VICTORY",
            Outcome::Defeat => "DEFEAT",
            Outcome::Stalemate => "STALEMATE",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CombatLog {
    pub player: Combatant,
    /// Everything on the other side, in the order they were written. Almost
    /// every fight has exactly one; `enemy()` is the shorthand for that case.
    pub enemies: Vec<Combatant>,
    /// The monsters fought, so the interface can lay their gear out beside
    /// yours without having to guess which rung the run has moved on to.
    pub specs: Vec<MonsterSpec>,
    pub entries: Vec<LogEntry>,
    pub outcome: Outcome,
    pub duration_ms: u32,
    /// Run gold the player spent during the fight. The run deducts it when
    /// the fight settles - the simulation never touches `Run::gold` itself.
    pub gold_spent: i32,
}

impl CombatLog {
    /// The creature you were fighting, when there was only the one - which is
    /// every fight except the handful an event sets up.
    pub fn enemy(&self) -> &Combatant {
        &self.enemies[0]
    }

    pub fn spec(&self) -> &MonsterSpec {
        &self.specs[0]
    }

    /// Is this a fight with more than one thing in it?
    pub fn is_brawl(&self) -> bool {
        self.enemies.len() > 1
    }

    /// A win with no fight in it. For the ladder picker and for tests, where
    /// what is under test is the settlement rather than the simulation.
    pub fn won_by_default(spec: &MonsterSpec) -> CombatLog {
        CombatLog {
            player: Combatant::player(Stats::base_character(), &[]),
            enemies: vec![Combatant::monster_at(spec, Difficulty::Medium)],
            specs: vec![*spec],
            entries: Vec::new(),
            outcome: Outcome::Victory,
            duration_ms: 0,
            gold_spent: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn who(&self, s: Side) -> &str {
        match s {
            Side::Player => &self.player.name,
            Side::Enemy => &self.enemy().name,
        }
    }

    /// One line of plain text, for the CLI and the on-screen log.
    pub fn describe(&self, e: &LogEntry) -> String {
        let t = format!("{:>5.1}s", e.at_ms as f32 / 1000.0);
        match &e.event {
            Event::Activate { side, item, .. } => {
                format!("{} {} activates {}", t, self.who(*side), item)
            }
            Event::Grew { side, amount, total } => format!(
                "{} {} grows {} tougher ({} max health)",
                t,
                self.who(*side),
                amount,
                total
            ),
            Event::Misfired { side, item } => {
                format!("{} {}'s {} misfires and does nothing", t, self.who(*side), item)
            }
            Event::Warded { side, item } => {
                format!("{} {}'s {} misses entirely", t, self.who(*side), item)
            }
            Event::Cast { side, paid, cost, remaining } => {
                if *paid {
                    format!(
                        "{} {} spends {} mana and casts in full ({} left)",
                        t,
                        self.who(*side),
                        cost,
                        remaining
                    )
                } else {
                    format!(
                        "{} {} has no mana - the spell lands weakly",
                        t,
                        self.who(*side)
                    )
                }
            }
            Event::ResourceCheck { side, what, cost, paid, remaining } => format!(
                "{} {} {} {} {} ({} left)",
                t,
                self.who(*side),
                if *paid { "spends" } else { "cannot pay" },
                cost,
                what,
                remaining
            ),
            Event::GainResource { side, what, amount, total } => {
                format!("{} {} gains {} {} ({})", t, self.who(*side), amount, what, total)
            }
            Event::Reflected { side, damage } => {
                format!("{} {} turns back {}", t, self.who(*side), damage)
            }
            Event::Fused { side, what, total, from, and } => format!(
                "{} {} fuses 1 {} ({}) - {} {} and {} {} left",
                t,
                self.who(*side),
                what,
                total,
                from.1,
                from.0,
                and.1,
                and.0
            ),
            Event::Watched { side, item, what } => {
                format!("{} {}'s {} has counted enough {}s", t, self.who(*side), item, what)
            }
            Event::Hit { by, damage, absorbed, target_health, target_armor } => {
                let soak = if *absorbed > 0 {
                    format!(" ({} soaked, {} armor left)", absorbed, target_armor)
                } else {
                    String::new()
                };
                format!(
                    "{} {} hits {} for {}{} -> {} hp",
                    t,
                    self.who(*by),
                    self.who(by.other()),
                    damage,
                    soak,
                    (*target_health).max(0)
                )
            }
            Event::MindHit { by, amount, target_max_health } => format!(
                "{} {} deals {} MIND damage -> max hp now {}",
                t,
                self.who(*by),
                amount,
                target_max_health
            ),
            Event::GainArmor { side, amount, total } => {
                format!("{} {} gains {} armor ({})", t, self.who(*side), amount, total)
            }
            Event::GainMana { side, amount, total } => {
                format!("{} {} gains {} mana ({})", t, self.who(*side), amount, total)
            }
            Event::ManaCheck { side, cost, paid, remaining } => {
                if *paid {
                    format!("{} {} spends {} mana ({} left)", t, self.who(*side), cost, remaining)
                } else {
                    format!(
                        "{} {} cannot pay {} mana (has {})",
                        t,
                        self.who(*side),
                        cost,
                        remaining
                    )
                }
            }
            Event::Stunned { on, item, duration_ms, aimed, .. } => format!(
                "{} {}{}'s {} is stunned for {:.1}s",
                t,
                if *aimed { "picks out " } else { "" },
                self.who(*on),
                item,
                *duration_ms as f32 / 1000.0
            ),
            Event::SuddenDeath { pct } => {
                format!("{} the fight turns - {}% of everyone, and rising", t, pct)
            }
            Event::Spent { side, amount, remaining } => format!(
                "{} {} spends {} fnorp ({} left)",
                t,
                self.who(*side),
                amount,
                remaining
            ),
            Event::Drained { on, what, amount, total } => format!(
                "{} {} loses {} {} ({} left)",
                t,
                self.who(*on),
                amount,
                what,
                total
            ),
            Event::Cursed { on, kind, duration_ms, stacks } => format!(
                "{} curse of {}{} on {} for {:.1}s",
                t,
                kind.name(),
                if *stacks > 1 { format!(" x{}", stacks) } else { String::new() },
                self.who(*on),
                *duration_ms as f32 / 1000.0
            ),
            Event::Burn { side, damage, health } => format!(
                "{} {} burns for {} -> {} hp",
                t,
                self.who(*side),
                damage,
                (*health).max(0)
            ),
            Event::Regen { side, amount, health } => {
                format!("{} {} regenerates {} -> {} hp", t, self.who(*side), amount, health)
            }
            Event::Hastened { side, item, by_ms } => format!(
                "{} {}'s {} hastened by {:.1}s",
                t,
                self.who(*side),
                item,
                *by_ms as f32 / 1000.0
            ),
            Event::Empowered { side, total, power_bonus } => format!(
                "{} {} empowered x{} (+{}.{:02}x power)",
                t,
                self.who(*side),
                total,
                power_bonus / 100,
                power_bonus % 100
            ),
            Event::Forking { side, total } => format!(
                "{} {} spell forking x{} (every cast lands {} times)",
                t,
                self.who(*side),
                total,
                total + 1
            ),
            Event::Shielded { side, total, reduction } => format!(
                "{} {} mana shield x{} (-{} per hit)",
                t,
                self.who(*side),
                total,
                reduction
            ),
            Event::Fell { side } => format!("{} {} falls!", t, self.who(*side)),
            Event::End { outcome } => format!("-- {} --", outcome.label()),
        }
    }
}

// ------------------------------------------------------------ simulate

/// Run the whole fight to completion.
///
/// Each [`TICK_MS`] slice, in strict order:
///   1. curses burn, then regeneration heals, on both sides
///   2. curse timers advance and expired curses drop
///   3. every item advances its cooldown — slowed if its owner is frosted —
///      and activates if full. The player's items resolve before the enemy's,
///      and within a side they resolve in loadout order.
///   4. deaths are checked
///
/// Nothing here consults a random number generator.
pub fn simulate(player_stats: Stats, profiles: &[ItemProfile], spec: &MonsterSpec) -> CombatLog {
    simulate_at(player_stats, profiles, spec, Difficulty::Easy)
}

pub fn simulate_at(
    player_stats: Stats,
    profiles: &[ItemProfile],
    spec: &MonsterSpec,
    difficulty: Difficulty,
) -> CombatLog {
    simulate_with_class(player_stats, profiles, spec, difficulty, &[])
}

/// The same, with the player's class applied. `Standing` powers are already
/// folded into `player_stats` by the run; the rest are rules the fight has to
/// know about.
pub fn simulate_with_class(
    player_stats: Stats,
    profiles: &[ItemProfile],
    spec: &MonsterSpec,
    difficulty: Difficulty,
    classes: &[crate::class::ClassDef],
) -> CombatLog {
    simulate_with_purse(player_stats, profiles, spec, difficulty, classes, 0)
}

/// The same, with a purse for `SpendGold` to reach into.
///
/// Split from `simulate_with_class` rather than added to it because only the
/// run has a purse: every test and every analysis tool fights without one, and
/// none of them should have to say so.
pub fn simulate_with_purse(
    player_stats: Stats,
    profiles: &[ItemProfile],
    spec: &MonsterSpec,
    difficulty: Difficulty,
    classes: &[crate::class::ClassDef],
    purse: i32,
) -> CombatLog {
    simulate_party(player_stats, profiles, std::slice::from_ref(spec), difficulty, classes, purse)
}

/// Fight everything in `specs` at once.
///
/// The player's single-target attacks land on whoever is at the front - the
/// first of them still standing - and every one of them acts against you
/// independently. It is over when all of them are down, or you are.
pub fn simulate_party(
    player_stats: Stats,
    profiles: &[ItemProfile],
    specs: &[MonsterSpec],
    difficulty: Difficulty,
    classes: &[crate::class::ClassDef],
    purse: i32,
) -> CombatLog {
    assert!(!specs.is_empty(), "a fight needs something to fight");
    let mut start_player = Combatant::player(player_stats, profiles);
    start_player.purse = purse;
    // Every class you hold applies at once. The fountains hand out different
    // classes, never the same one twice, so two powers never fight over the
    // same field.
    for c in classes {
        match c.power {
            crate::class::ClassPower::SlowTime(n) => start_player.slow_time = n,
            crate::class::ClassPower::Overflowing(n) => start_player.overflowing = n,
            crate::class::ClassPower::Leeching(pct) => start_player.leech = pct,
            crate::class::ClassPower::Standing(_) => {}
            crate::class::ClassPower::Echo(n) => start_player.echo_every = n,
            crate::class::ClassPower::Bastion(pct) => start_player.bastion = pct,
            crate::class::ClassPower::Contagion(n) => start_player.contagion = n,
            crate::class::ClassPower::Guilt => start_player.no_regen = true,
            // The two that stack. Every other arm here assigns, because the
            // fountains never hand out the same class twice; a town hands out
            // the same one over and over on purpose, so these add.
            // Recycler is a board rule, not a fight rule: it scales adjacency
            // bonuses, which are already in the stats and the item profiles
            // this fight was handed. See `Loadout::adjacency_pct`.
            crate::class::ClassPower::Recycler { .. } => {}
            crate::class::ClassPower::Piety { faith } => start_player.faith += faith,
            crate::class::ClassPower::Tired { mana } => start_player.mana -= mana,
            crate::class::ClassPower::Ticket { nth } => start_player.warded_every = nth,
            crate::class::ClassPower::Trundle { slower, armour } => {
                start_player.slower_pct = slower;
                start_player.armour_pct = armour;
            }
            crate::class::ClassPower::Longhaul { per_second } => {
                start_player.haste_per_s = per_second;
            }
            crate::class::ClassPower::Reprisal(n) => start_player.reprisal = n,
            crate::class::ClassPower::Riposte(ms) => start_player.riposte = ms,
            crate::class::ClassPower::Momentum(n) => start_player.momentum = n,
            crate::class::ClassPower::Resonance(n) => start_player.resonance = n,
            crate::class::ClassPower::Transmute(pct) => start_player.transmute = pct,
            crate::class::ClassPower::Adaptable(n) => start_player.adaptable = n,
            crate::class::ClassPower::Untimely(n) => start_player.untimely = n,
            crate::class::ClassPower::Cascade(ms) => start_player.cascade = ms,
            crate::class::ClassPower::Consecrate(pct) => start_player.consecrate = pct,
            crate::class::ClassPower::Bloodscent(n) => start_player.bloodscent = n,
            crate::class::ClassPower::Confluence(pct) => start_player.confluence = pct,
            // Split the wisdom: every item takes a share of the best
            // multiplier on the board on top of its own. Done here rather than
            // in the profile because it is a property of the whole board, and
            // the profile only knows about one item.
            crate::class::ClassPower::Avenged(n) => start_player.rage += n,
            crate::class::ClassPower::Splintered(pct) => {
                let best = start_player.items.iter().map(|i| i.power).max().unwrap_or(100);
                let share = (best - 100).max(0) * pct / 100;
                for it in &mut start_player.items {
                    it.power += share;
                }
            }
        }
    }
    let start_player = start_player;
    let start_enemies: Vec<Combatant> =
        specs.iter().map(|m| Combatant::monster_at(m, difficulty)).collect();
    let mut p = start_player.clone();
    let mut foes: Vec<Combatant> = start_enemies.clone();
    let mut log: Vec<LogEntry> = Vec::new();
    // Reported once each, as they go down.
    let mut fallen: Vec<usize> = Vec::new();

    // Everyone in the fight, player first, so the loops below read the same
    // whether there is one thing across the table or three.
    let everyone = |foes: &[Combatant]| -> Vec<Ref> {
        std::iter::once(Ref::PLAYER).chain((0..foes.len()).map(Ref::foe)).collect()
    };

    // What each side walks in already holding. Everything else starts a fight
    // at zero and earns its way up, which makes the opening of every fight
    // look the same whatever you are wearing; this is the gear that does not.
    for me in everyone(&foes) {
        let opening: Vec<(usize, Action)> = pick(&mut p, &mut foes, me)
            .items
            .iter()
            .enumerate()
            .flat_map(|(i, it)| {
                it.triggers
                    .iter()
                    .filter_map(move |t| match t {
                        Trigger::OnBattleStart(a) => Some((i, *a)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        for (idx, action) in opening {
            apply(&mut p, &mut foes, me, action, 0, &mut log, Some(idx));
        }
    }
    let mut outcome = Outcome::Stalemate;
    let mut t: u32 = 0;

    'fight: while t < MAX_DURATION_MS {
        t += TICK_MS;

        // 0. Slow time: whatever was queued arrives a slice at a time.
        for c in std::iter::once(&mut p).chain(foes.iter_mut()) {
            if c.pending.is_empty() {
                continue;
            }
            let mut still = Vec::new();
            let mut arriving = 0;
            for (amount, left) in std::mem::take(&mut c.pending) {
                let slice = (amount * TICK_MS as i32 / SLOW_TIME_MS as i32).max(1).min(amount);
                arriving += slice;
                let rest = amount - slice;
                let left = left.saturating_sub(TICK_MS);
                if rest > 0 && left > 0 {
                    still.push((rest, left));
                } else if rest > 0 {
                    arriving += rest;
                }
            }
            c.pending = still;
            if arriving > 0 {
                let absorbed = arriving.min(c.armor.max(0));
                c.armor -= absorbed;
                c.health -= arriving - absorbed;
            }
        }

        // 1. Damage over time, then healing.
        for me in everyone(&foes) {
            let side = me.side;
            let who = me.who as u8;
            let c = pick(&mut p, &mut foes, me);
            c.dot_milli += c.curses.dot_millidamage_per_tick();
            let whole = c.dot_milli / 1000;
            if whole > 0 {
                c.dot_milli %= 1000;
                c.health -= whole;
                c.burn_acc += whole;
            }
            // Report burn once a second, or immediately if it just killed
            // them, rather than a line per tick.
            c.burn_timer += TICK_MS;
            if c.burn_acc > 0 && (c.burn_timer >= BURN_REPORT_MS || c.health <= 0) {
                let (dmg, hp) = (c.burn_acc, c.health);
                c.burn_acc = 0;
                c.burn_timer = 0;
                log.push(LogEntry { who, at_ms: t, event: Event::Burn { side, damage: dmg, health: hp } });
            }
            let regen = c.effective_regen();
            if regen > 0 && c.health < c.max_health && !c.no_regen {
                c.regen_milli += regen * TICK_MS as i32;
                let heal = (c.regen_milli / 1000).min(c.max_health - c.health);
                if heal > 0 {
                    c.regen_milli %= 1000;
                    c.health += heal;
                    let hp = c.health;
                    log.push(LogEntry {
                        who,
                        at_ms: t,
                        event: Event::Regen { side, amount: heal, health: hp },
                    });
                }
            }
        }
        // 1b. Sudden death. A fight that has gone on this long ends itself,
        // and it ends itself for everybody at once - straight off health,
        // past armour and resistance, because a wall you can hide behind for
        // ever is exactly what this exists to stop.
        if t >= SUDDEN_DEATH_MS && t % 1000 == 0 {
            let second = ((t - SUDDEN_DEATH_MS) / 1000 + 1) as i32;
            log.push(LogEntry { who: 0, at_ms: t, event: Event::SuddenDeath { pct: second } });
            for c in std::iter::once(&mut p).chain(foes.iter_mut()) {
                let bite = (c.max_health * second / 100).max(1);
                c.health -= bite;
            }
        }

        if check_down(&p, &foes, t, &mut log, &mut outcome, &mut fallen) {
            break 'fight;
        }

        // 2. Curse timers.
        p.curses.tick();
        for f in foes.iter_mut() {
            f.curses.tick();
        }

        // 3. Cooldowns and activations.
        //
        // A foe that is already down does not get a turn, but the loop still
        // walks past it: the living ones keep their own item order, which is
        // what makes a fight replay identically.
        for me in everyone(&foes) {
            let side = me.side;
            if pick(&mut p, &mut foes, me).is_down() {
                continue;
            }
            let count = pick(&mut p, &mut foes, me).items.len();
            for idx in 0..count {
                let ready = {
                    let c = pick(&mut p, &mut foes, me);
                    // Frost stretches the cooldown by slowing how fast the
                    // bar fills, rather than by rewriting the cooldown. It is
                    // a property of the fighter, so it is read before the item.
                    let slow = c.curses.slow_pct();
                    let slower = c.slower_pct;
                    // The long haul: everything winds up as the fight drags,
                    // to twice speed and no further.
                    let haste = (c.haste_per_s * (t / 1000) as i32).clamp(0, 100);
                    let item = &mut c.items[idx];
                    // A stun stops this item's bar dead. Not a slow: it does
                    // not advance at all, and what was part-way through stays
                    // part-way through, so it resumes rather than starting
                    // over. Only this item - the rest of the kit plays on.
                    if item.stun_ms > 0 {
                        item.stun_ms = item.stun_ms.saturating_sub(TICK_MS);
                        false
                    } else {
                        let step = (TICK_MS as i32 * (100 - slow) / 100 * (100 - slower) / 100
                            * (100 + haste)
                            / 100)
                            .max(1) as u32;
                        item.progress_ms += step;
                        if item.progress_ms >= item.cooldown_ms {
                            item.progress_ms -= item.cooldown_ms;
                            true
                        } else {
                            false
                        }
                    }
                };
                if ready {
                    // A misfire eats the activation itself: the cooldown has
                    // already come round, and nothing comes of it.
                    let fizzled = {
                        let c = pick(&mut p, &mut foes, me);
                        c.misfire_count = c.misfire_count.wrapping_add(1);
                        c.curses.misfires(c.misfire_count)
                    };
                    if fizzled {
                        let name = pick(&mut p, &mut foes, me).items[idx].name.clone();
                        let front = aim_of(&foes, p.aim);
                        log.push(LogEntry {
                            who: me.logged_as(front),
                            at_ms: t,
                            event: Event::Misfired { side, item: name },
                        });
                        continue;
                    }
                    // Ticket to Ride: every nth thing they swing at you comes
                    // to nothing. Eaten here rather than at each damage site
                    // because a warded attack lands nothing at all - no hit,
                    // no curse, no drain - and there are a dozen ways for one
                    // activation to reach you.
                    if side == Side::Enemy && p.warded_every > 0 {
                        let c = pick(&mut p, &mut foes, me);
                        c.warded_count = c.warded_count.wrapping_add(1);
                        let warded = c.warded_count % p.warded_every == 0;
                        if warded {
                            let name = pick(&mut p, &mut foes, me).items[idx].name.clone();
                            let front = aim_of(&foes, p.aim);
                            log.push(LogEntry {
                                who: me.logged_as(front),
                                at_ms: t,
                                event: Event::Warded { side, item: name },
                            });
                            continue;
                        }
                    }
                    activate(&mut p, &mut foes, me, idx, t, &mut log);
                    if check_down(&p, &foes, t, &mut log, &mut outcome, &mut fallen) {
                        break 'fight;
                    }
                }
            }
        }
    }

    log.push(LogEntry { who: 0, at_ms: t, event: Event::End { outcome } });
    // What the purse lost over the fight, for the run to charge afterwards.
    let spent_from_purse = purse - p.purse;
    CombatLog {
        player: start_player,
        enemies: start_enemies,
        specs: specs.to_vec(),
        entries: log,
        outcome,
        duration_ms: t,
        gold_spent: spent_from_purse,
    }
}

/// One combatant in a fight: the player, or the nth foe.
///
/// `Side` stays two-valued because the *rules* are two-sided - you and them -
/// and only the far side can have more than one body in it. This pairs the
/// side with which body, and is what every helper threads instead of a bare
/// `Side`.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct Ref {
    side: Side,
    who: usize,
}

impl Ref {
    const PLAYER: Ref = Ref { side: Side::Player, who: 0 };

    fn foe(who: usize) -> Ref {
        Ref { side: Side::Enemy, who }
    }

    /// The far side of this exchange. For a foe that is always the player;
    /// for the player it is whoever is at the front of the queue.
    fn other(self, front: usize) -> Ref {
        match self.side {
            Side::Player => Ref::foe(front),
            Side::Enemy => Ref::PLAYER,
        }
    }

    /// What the log records this entry as being about. The player is always
    /// singular, so an entry is about a foe either way: the one acting, or the
    /// one being acted upon.
    fn logged_as(self, front: usize) -> u8 {
        match self.side {
            Side::Player => front as u8,
            Side::Enemy => self.who as u8,
        }
    }
}

/// Whoever the player's next single-target attack lands on.
///
/// Every attack moves the aim along, so a brawl is whittled down at roughly
/// one rate rather than one at a time. That matters for what a two-creature
/// fight *is*: focusing the front one down would make a brawl a queue, where
/// killing the first thing halves the incoming damage and the second half of
/// the fight is easier than the first. Spreading it means both of them are
/// hitting you until nearly the end, which is what makes two of something
/// worse than one of something twice the size.
///
/// Skips anything already down, and never gets stuck: if they are all down
/// the fight is over before this is asked again.
fn aim_of(foes: &[Combatant], cursor: usize) -> usize {
    let n = foes.len();
    (0..n)
        .map(|k| (cursor + k) % n)
        .find(|&i| !foes[i].is_down())
        .unwrap_or(0)
}

fn pick<'a>(p: &'a mut Combatant, foes: &'a mut [Combatant], r: Ref) -> &'a mut Combatant {
    match r.side {
        Side::Player => p,
        Side::Enemy => &mut foes[r.who.min(foes.len().saturating_sub(1))],
    }
}

fn check_down(
    p: &Combatant,
    foes: &[Combatant],
    t: u32,
    log: &mut Vec<LogEntry>,
    outcome: &mut Outcome,
    fallen: &mut Vec<usize>,
) -> bool {
    // Each foe is reported the once, as it goes down, so a brawl reads like a
    // brawl rather than announcing the same corpse every tick.
    for (i, f) in foes.iter().enumerate() {
        if f.is_down() && !fallen.contains(&i) {
            fallen.push(i);
            log.push(LogEntry { who: i as u8, at_ms: t, event: Event::Fell { side: Side::Enemy } });
        }
    }
    let cleared = foes.iter().all(|f| f.is_down());
    let fell = p.is_down();
    if fell {
        log.push(LogEntry { who: 0, at_ms: t, event: Event::Fell { side: Side::Player } });
    }

    match (cleared, fell) {
        (false, false) => false,
        (true, false) => {
            *outcome = Outcome::Victory;
            true
        }
        (false, true) => {
            *outcome = Outcome::Defeat;
            true
        }
        // Everyone went down on the same tick, which sudden death makes a
        // real possibility rather than a curiosity. Whoever is less far past
        // zero takes it, and a dead heat goes to the player: the fight was
        // even, and an even fight should not cost a life.
        (true, true) => {
            let best = foes.iter().map(|f| f.health).max().unwrap_or(i32::MIN);
            *outcome = if p.health >= best { Outcome::Victory } else { Outcome::Defeat };
            true
        }
    }
}

/// Resolve one item firing: its flat effects, then its triggers in order.
/// Which item a stun takes out.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum StunAim {
    /// Whichever one it happens to catch. This is what a plain curse of stun
    /// does, and not knowing which is most of what keeps it fair.
    Unaimed,
    /// The best thing they own, by the same effectiveness rating the shop
    /// prices gear with. Costs more, and it should - picking the target is
    /// worth more than the stun.
    Strongest,
}

/// Land a stun on one of `victim`'s items and return which, with how long for.
///
/// The choice is deterministic, because the whole engine is: every test in the
/// suite replays a fight and expects the same answer, and a real roll would
/// end that. It is still unpredictable from the far side of the screen, which
/// is the property that actually matters - the same trade `Misfire` makes by
/// counting activations rather than rolling for them.
///
/// Nothing lands on an item that is already stopped for longer than this stun
/// would stop it, when there is a live one to hit instead: a chain of stuns
/// should spread across the kit, not bury one item.
fn land_curse(
    victim: &mut Combatant,
    on: Ref,
    kind: CurseKind,
    aim: StunAim,
    t: u32,
    log: &mut Vec<LogEntry>,
) {
    let who = on.who as u8;
    let on = on.side;
    if kind == CurseKind::Stun {
        if let Some((index, ms)) = land_stun(victim, aim, t) {
            let item = victim.items[index].name.clone();
            let aimed = aim == StunAim::Strongest;
            log.push(LogEntry {
                who,
                at_ms: t,
                event: Event::Stunned { on, index, item, duration_ms: ms, aimed },
            });
        }
        return;
    }
    let ms = victim.curses.apply(kind, victim.curse_resist);
    if ms > 0 {
        let stacks = victim.curses.stacks_of(kind);
        log.push(LogEntry {
            who,
            at_ms: t,
            event: Event::Cursed { on, kind, duration_ms: ms, stacks },
        });
    }
}

fn land_stun(victim: &mut Combatant, aim: StunAim, at_ms: u32) -> Option<(usize, u32)> {
    let duration = CurseKind::Stun.landing_ms(victim.curse_resist);
    if duration == 0 || victim.items.is_empty() {
        return None;
    }
    victim.stun_count = victim.stun_count.wrapping_add(1);

    let idx = match aim {
        StunAim::Strongest => victim
            .items
            .iter()
            .enumerate()
            // Among equals take the one still running: stunning what is
            // already stopped is the one outcome an aimed stun must not have.
            .max_by_key(|(_, it)| (it.rating, it.stun_ms == 0))
            .map(|(i, _)| i)?,
        StunAim::Unaimed => {
            let n = victim.items.len();
            // A cheap integer hash of the fight's own state. Time alone
            // clusters, because stuns arrive on cooldown boundaries.
            let mix = (at_ms as u64)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add((victim.stun_count as u64).wrapping_mul(0x2545_F491_4F6C_DD1D));
            let start = (mix >> 33) as usize % n;
            // Walk from there to the first item that is not already stopped,
            // falling back to the original pick if every one of them is.
            (0..n)
                .map(|k| (start + k) % n)
                .find(|&i| victim.items[i].stun_ms == 0)
                .unwrap_or(start)
        }
    };

    let item = &mut victim.items[idx];
    // Stacks pile onto that item's clock rather than refreshing it, so a
    // second stun landing on the same item is worth something.
    item.stun_ms = (item.stun_ms + duration).min(STUN_CAP_MS);
    Some((idx, item.stun_ms))
}

fn activate(
    p: &mut Combatant,
    foes: &mut Vec<Combatant>,
    me: Ref,
    idx: usize,
    t: u32,
    log: &mut Vec<LogEntry>,
) {
    let front = aim_of(foes, p.aim);
    let side = me.side;
    // Taken before the local rebindings below shadow `me` with a combatant.
    let who = me.logged_as(front);
    let mut item = pick(p, foes, me).items[idx].clone();

    // A spell swaps in the payload whose turn it is. A book has bound one and
    // casts it every time; a crystal ball cycles through the two or three it
    // holds, so the same item does something different each time it comes
    // round. The index lives on the combatant's copy, not this clone.
    // Echo: every nth activation runs its payload a second time.
    let echoes = {
        let me = pick(p, foes, me);
        me.activations += 1;
        me.echo_every > 0 && me.activations % me.echo_every == 0
    };
    let mut cast_name = None;
    if !item.casts.is_empty() {
        let n = item.casts.len();
        let which = item.cast_index % n;
        let cast = item.casts[which].clone();
        item.physical_damage = cast.stats.physical_damage;
        item.magic_damage = cast.stats.magic_damage;
        item.rage = cast.stats.rage;
        item.faith = cast.stats.faith;
        item.nature = cast.stats.nature;
        item.mind = cast.stats.mind;
        item.armor = cast.stats.armor;
        item.mana = cast.stats.mana;
        item.triggers = cast.triggers;
        // The spells that did not come up this turn still answer the one that
        // did. This is what makes a ball worth more than its spells apart:
        // only a crystal ball holds several, so only a ball can pay this out.
        for (i, other) in item.casts.iter().enumerate() {
            if i == which {
                continue;
            }
            for trig in &other.triggers {
                if let Trigger::OnOtherCast(a) = trig {
                    item.triggers.push(Trigger::OnActivate(*a));
                }
            }
        }
        if n > 1 {
            cast_name = Some(cast.name);
        }
        // A ball speaks with two voices. This is what a ball IS - a book binds
        // one spell and casts it every time, and if a ball only ever cast one
        // too then holding three of them bought nothing but variety. The
        // second is whichever is next in the cycle, so which pair you get
        // still changes each time it comes round.
        let extra = (BALL_VOICES - 1) as usize;
        for k in 0..extra.min(n.saturating_sub(1)) {
            let also = &item.casts[(which + 1 + k) % n];
            item.physical_damage += also.stats.physical_damage;
            item.magic_damage += also.stats.magic_damage;
            item.rage += also.stats.rage;
            item.faith += also.stats.faith;
            item.nature += also.stats.nature;
            item.mind += also.stats.mind;
            item.armor += also.stats.armor;
            item.mana += also.stats.mana;
            item.triggers.extend(also.triggers.iter().copied());
        }
        pick(p, foes, me).items[idx].cast_index = (which + 1) % n;

        // A spell has two intensities. Paid for, it lands in full; unpaid, it
        // still goes off but weakly. Mana stops being a thing some gear
        // happens to grant and becomes the difference between a spell that
        // works and a spell that merely happens.
        //
        // One price per activation, covering every voice: a ball is meant to
        // be the committed choice, and charging it twice for being one would
        // undo that.
        let paid = {
            let me = pick(p, foes, me);
            if me.mana >= SPELL_MANA_COST {
                me.mana -= SPELL_MANA_COST;
                true
            } else {
                false
            }
        };
        let scale = if paid { EMPOWERED_CAST_PCT } else { WEAK_CAST_PCT };
        for v in [
            &mut item.physical_damage,
            &mut item.magic_damage,
            &mut item.mind,
            &mut item.armor,
        ] {
            *v = *v * scale / 100;
        }
        let remaining = pick(p, foes, me).mana;
        log.push(LogEntry {
            who,
            at_ms: t,
            event: Event::Cast { side, paid, cost: SPELL_MANA_COST, remaining },
        });
    }

    log.push(LogEntry {
        who,
        at_ms: t,
        event: Event::Activate {
            side,
            item: match cast_name {
                Some(spell) => format!("{} ({})", item.name, spell),
                None => item.name.clone(),
            },
            index: idx,
        },
    });

    // Weapons swing; everything else just does its job. A monster's attacks
    // have no slot and always count as weapons.
    let is_weapon = item.slot.map(|s| s == SlotKind::Weapon).unwrap_or(true);
    if is_weapon {
        // Strength reaches every weapon; power does not reach past the one
        // carrying it. Empowerment is the exception and is meant to be: it is
        // bought with mana, at five hundredths a stack a point, and it applies
        // to whatever is swinging.
        let (strength, empower) = {
            let me = pick(p, foes, me);
            (me.strength, me.empowerment as i32 * 5 * me.mana.max(0))
        };
        // The wearer's power, plus whatever ink is bound into this item alone.
        // Rage held sharpens the physical half.
        let (rage, phys_pierce, magic_pierce) = {
            let me = pick(p, foes, me);
            (me.held_bonus().physical_damage, me.physical_pierce, me.magic_pierce)
        };
        // The item's own numbers already carry its power - it was applied
        // when the profile was built, so the card and the fight agree. What
        // the wearer brings does not, so it picks the multiplier up here.
        let mult = |flat: i32| -> i32 { ((flat as i64) * (100 + empower) as i64 / 100).max(0) as i32 };
        let from_wearer =
            (((rage + strength) as i64 * item.power as i64) / 100).max(0) as i32;
        let physical = mult(item.physical_damage + from_wearer);
        // Transmute: part of the iron lands again as magic.
        let transmute = pick(p, foes, me).transmute;
        let magic = mult(item.magic_damage) + physical * transmute / 100;
        // Momentum: the longer the fight runs, the harder you swing.
        let momentum = pick(p, foes, me).momentum * (t / 1000) as i32;
        let physical =
            physical + mult((((momentum as i64) * item.power as i64) / 100) as i32);
        // A fork copies the cast, and only a cast: a blade swings once
        // however many stacks are up.
        let forks = if item.casts.is_empty() { 0 } else { pick(p, foes, me).forking };
        let reps: u32 = if echoes { 2 } else { 1 } * (1 + forks);

        // The log reports the swing, not what survived the defences: a hit
        // that is turned aside completely still has to show up, or a player
        // stacking resistance sees nothing happening at all.
        let swing = physical + magic;
        // One blow per repetition, each aimed afresh. An echo or a fork is
        // another attack, so it takes the next one along - and a line of its
        // own in the log, which is also more honest about what happened than
        // folding two blows into one number was.
        for _ in 0..reps {
            let aim = aim_of(foes, p.aim);
            let at = me.other(aim);
            let mut absorbed_total = 0;
            for (amount, kind, pierce) in [
                (physical, DamageType::Physical, phys_pierce),
                (magic, DamageType::Magic, magic_pierce),
            ] {
                if amount <= 0 {
                    continue;
                }
                let target = pick(p, foes, at);
                let (absorbed, _) = target.take_typed(amount, kind, pierce);
                absorbed_total += absorbed;
            }
            // Reflection. What the armour ate is turned back on whoever swung
            // it, which is why this is the body's attack and nothing else's: it
            // needs the blow to land and be absorbed first, so it pays nothing
            // to a board that dies quickly and everything to one built to be
            // hit. Taken as physical, and it cannot itself be reflected - the
            // return is dealt directly rather than back through this path, so
            // two reflecting boards cannot bounce a hit between them for ever.
            let pct = pick(p, foes, at).reflect;
            if pct > 0 && absorbed_total > 0 {
                let back = absorbed_total * pct / 100;
                if back > 0 {
                    let swinger = pick(p, foes, me);
                    swinger.health -= back;
                    log.push(LogEntry {
                        who: me.logged_as(aim),
                        at_ms: t,
                        event: Event::Reflected { side: at.side, damage: back },
                    });
                }
            }
            if swing > 0 {
                let target = pick(p, foes, at);
                let (hp, ar) = (target.health, target.armor);
                log.push(LogEntry {
                    who: me.logged_as(aim),
                    at_ms: t,
                    event: Event::Hit {
                        by: side,
                        damage: swing,
                        absorbed: absorbed_total,
                        target_health: hp,
                        target_armor: ar,
                    },
                });
            }
            // Next blow goes to the next one along.
            if me.side == Side::Player && foes.len() > 1 {
                p.aim = aim + 1;
            }
        }
        // Leeching: a share of what you dealt comes back.
        let leech = pick(p, foes, me).leech;
        if leech > 0 && swing > 0 {
            let me = pick(p, foes, me);
            let back = (swing * reps as i32) * leech / 100;
            me.health = (me.health + back).min(me.max_health);
        }
    }

    if let Some(kind) = item.curse {
        apply(p, foes, me, Action::Curse { kind, target: Target::Enemy }, t, log, Some(idx));
    }

    if item.mind > 0 {
        let target = pick(p, foes, me.other(front));
        let dealt = target.take_mind(item.mind);
        let mh = target.max_health;
        if dealt > 0 {
            log.push(LogEntry {
                who,
                at_ms: t,
                event: Event::MindHit { by: side, amount: dealt, target_max_health: mh },
            });
        }
    }

    if item.armor > 0 {
        let me = pick(p, foes, me);
        let got = me.gain_armor(item.armor);
        let total = me.armor;
        log.push(LogEntry {
            who,
            at_ms: t,
            event: Event::GainArmor { side, amount: got, total },
        });
    }

    if item.mana > 0 {
        let me = pick(p, foes, me);
        me.mana += item.mana;
        let total = me.mana;
        log.push(LogEntry { who, at_ms: t, event: Event::GainMana { side, amount: item.mana, total } });
    }

    let banked = pick(p, foes, me).adaptable;
    if banked > 0 {
        let me = pick(p, foes, me);
        me.mana += banked;
        me.rage += banked;
        me.faith += banked;
        me.nature += banked;
    }
    // Riposte: watching them act gives your own gear a nudge.
    {
        let ms = pick(p, foes, me.other(front)).riposte;
        if ms > 0 {
            for it in &mut pick(p, foes, me.other(front)).items {
                it.progress_ms += ms;
            }
        }
    }

    for (amount, label) in [(item.rage, "rage"), (item.faith, "faith"), (item.nature, "nature")] {
        if amount > 0 {
            let me = pick(p, foes, me);
            match label {
                "rage" => me.rage += amount,
                "faith" => me.faith += amount,
                _ => me.nature += amount,
            }
            let total = match label {
                "rage" => me.rage,
                "faith" => me.faith,
                _ => me.nature,
            };
            log.push(LogEntry {
                who,
                at_ms: t,
                event: Event::GainResource { side, what: label, amount, total },
            });
        }
    }

    // A repeat is expanded here rather than in the match below, so the thing
    // being repeated stays an ordinary trigger and every arm keeps working.
    let mut firing: Vec<Trigger> = Vec::with_capacity(item.triggers.len());
    for trigger in &item.triggers {
        match *trigger {
            Trigger::PerAdjacentEmpty(inner) => {
                for _ in 0..item.open_cells {
                    firing.push(*inner);
                }
            }
            other => firing.push(other),
        }
    }

    for trigger in &firing {
        match *trigger {
            Trigger::OnActivate(action) => apply(p, foes, me, action, t, log, Some(idx)),
            Trigger::SpendGold { cost, budget, on_success } => {
                let paid = {
                    let me = pick(p, foes, me);
                    let it = &mut me.items[idx];
                    // Two ways to come up short, and they are not the same:
                    // the budget is the promise the piece made, the purse is
                    // the money you actually have.
                    if it.gold_spent + cost <= budget && me.purse >= cost {
                        it.gold_spent += cost;
                        it.gold_paid += 1;
                        let times = it.gold_paid;
                        me.purse -= cost;
                        Some((times, me.purse))
                    } else {
                        None
                    }
                };
                if let Some((times, left)) = paid {
                    log.push(LogEntry {
                        who,
                        at_ms: t,
                        event: Event::Spent { side, amount: cost, remaining: left },
                    });
                    // Harder every time it pays. `scaled` touches outcomes and
                    // never costs, so the price stays flat while the payout
                    // climbs - which is the whole shape of the thing.
                    let grown = on_success.scaled(100 * times as i32);
                    apply(p, foes, me, grown, t, log, Some(idx));
                }
            }
            Trigger::SpendMana { cost, on_success, on_failure } => {
                let paid = {
                    let me = pick(p, foes, me);
                    if me.mana >= cost {
                        me.mana -= cost;
                        true
                    } else {
                        false
                    }
                };
                let remaining = pick(p, foes, me).mana;
                log.push(LogEntry {
                    who,
                    at_ms: t,
                    event: Event::ManaCheck { side, cost, paid, remaining },
                });
                apply(p, foes, me, if paid { on_success } else { on_failure }, t, log, Some(idx));
            }
            Trigger::Consume { what, each, per } => {
                // Takes the whole pool and pays out by the handful. The
                // remainder below one handful is spent too - the trigger is
                // "empty your reserve", not "spend a multiple of `each`".
                let (held, times) = {
                    let me = pick(p, foes, me);
                    let held = me.pool(what).max(0);
                    let times = held / each.max(1);
                    if times > 0 {
                        me.set_pool(what, 0);
                        // Confluence pays on this too: what one pool spends,
                        // the others drink.
                        let back = me.confluence * held / 100;
                        if back > 0 {
                            for other in
                                [Resource::Mana, Resource::Rage, Resource::Faith, Resource::Nature]
                            {
                                if other != what {
                                    let total = me.pool(other) + back;
                                    me.set_pool(other, total);
                                }
                            }
                        }
                    }
                    (held, times)
                };
                if times > 0 {
                    log.push(LogEntry {
                        who,
                        at_ms: t,
                        event: Event::ResourceCheck {
                            side,
                            what: what.name(),
                            cost: held,
                            paid: true,
                            remaining: 0,
                        },
                    });
                    for _ in 0..times {
                        apply(p, foes, me, per, t, log, Some(idx));
                    }
                }
            }
            Trigger::Spend { what, cost, on_success, on_failure } => {
                let paid = {
                    let me = pick(p, foes, me);
                    let held = me.pool(what);
                    if held >= cost {
                        me.set_pool(what, held - cost);
                        // Confluence: what one pool spends, the others drink.
                        let back = me.confluence * cost / 100;
                        if back > 0 {
                            for other in
                                [Resource::Mana, Resource::Rage, Resource::Faith, Resource::Nature]
                            {
                                if other != what {
                                    let total = me.pool(other) + back;
                                    me.set_pool(other, total);
                                }
                            }
                        }
                        true
                    } else {
                        false
                    }
                };
                let remaining = pick(p, foes, me).pool(what);
                log.push(LogEntry {
                    who,
                    at_ms: t,
                    event: Event::ResourceCheck { side, what: what.name(), cost, paid, remaining },
                });
                apply(p, foes, me, if paid { on_success } else { on_failure }, t, log, Some(idx));
            }
            Trigger::PerAdjacentItem { action, same_slot_only: _ } => {
                for _ in 0..item.adjacent_assembled_same_slot {
                    apply(p, foes, me, action, t, log, Some(idx));
                }
            }
            // Already expanded above; a nested one is not authored.
            Trigger::PerAdjacentEmpty(_) => {}
            // Fired before the first tick, not on the cooldown.
            Trigger::OnBattleStart(_) => {}
            // These wait for someone else to act.
            Trigger::OnAdjacentActivate(_)
            | Trigger::OnAlignedActivate(_)
            | Trigger::OnDiagonalActivate(_)
            | Trigger::OnOtherCast(_) => {}
            // A watcher does not act on its own cadence either. It is fed by
            // `notify_watchers` as the events it counts go past.
            Trigger::Watch { .. } => {}
        }
    }

    // Untimely: an Oracle reaches past the gear and at the clock behind it.
    let untimely = pick(p, foes, me).untimely;
    if untimely > 0 {
        let due = {
            let me = pick(p, foes, me);
            me.untimely_count = me.untimely_count.wrapping_add(1);
            me.untimely_count % untimely == 0
        };
        if due {
            for kind in [CurseKind::Stun, CurseKind::Misfire] {
                let victim = pick(p, foes, me.other(front));
                land_curse(victim, me.other(front), kind, StunAim::Unaimed, t, log);
            }
        }
    }

    // Cascade: everything else moves a little closer to firing. Never the item
    // that just went off, or a single fast item would wind itself up forever.
    let cascade = pick(p, foes, me).cascade;
    if cascade > 0 {
        let me = pick(p, foes, me);
        for (i, it) in me.items.iter_mut().enumerate() {
            if i != idx {
                it.progress_ms =
                    (it.progress_ms + cascade).min(it.cooldown_ms.saturating_sub(1));
            }
        }
    }

    // Finally, let the neighbours react. A reaction never emits an activation
    // of its own, so two items that react to each other cannot loop.
    notify_reactors(p, foes, me, idx, t, log);
}

/// Run every reaction owed to `actor_idx` firing.
fn notify_reactors(
    p: &mut Combatant,
    foes: &mut Vec<Combatant>,
    me: Ref,
    actor_idx: usize,
    t: u32,
    log: &mut Vec<LogEntry>,
) {
    let count = pick(p, foes, me).items.len();
    for j in 0..count {
        if j == actor_idx {
            continue;
        }
        let (touches, lines_up, corners, triggers) = {
            let c = pick(p, foes, me);
            let it = &c.items[j];
            (
                it.adjacent_items.contains(&actor_idx),
                it.aligned_items.contains(&actor_idx),
                it.diagonal_items.contains(&actor_idx),
                it.triggers.clone(),
            )
        };
        // Resonance doubles the answer, not the question: a reaction still
        // never emits an activation, so two items answering each other cannot
        // loop however loud it gets.
        let times = pick(p, foes, me).resonance.max(1);
        for tr in &triggers {
            for _ in 0..times {
                match *tr {
                    Trigger::OnAdjacentActivate(a) if touches => {
                        apply(p, foes, me, a, t, log, Some(j))
                    }
                    Trigger::OnAlignedActivate(a) if lines_up => {
                        apply(p, foes, me, a, t, log, Some(j))
                    }
                    Trigger::OnDiagonalActivate(a) if corners => {
                        apply(p, foes, me, a, t, log, Some(j))
                    }
                    _ => {}
                }
            }
        }
    }

    // And let the watchers count it. Separate pass so a reaction that fires
    // this tick is not itself counted as an activation - a watcher counts
    // items coming round, not the answers they provoke.
    for j in 0..count {
        if j == actor_idx {
            continue;
        }
        let (touches, lines_up, corners) = {
            let it = &pick(p, foes, me).items[j];
            (
                it.adjacent_items.contains(&actor_idx),
                it.aligned_items.contains(&actor_idx),
                it.diagonal_items.contains(&actor_idx),
            )
        };
        for what in [
            Watched::AnyActivation,
            Watched::AdjacentActivation,
            Watched::AlignedActivation,
            Watched::DiagonalActivation,
        ] {
            let saw = match what {
                Watched::AnyActivation => true,
                Watched::AdjacentActivation => touches,
                Watched::AlignedActivation => lines_up,
                Watched::DiagonalActivation => corners,
                Watched::CurseApplied => false,
            };
            if saw {
                tick_watchers(p, foes, me, j, what, t, log);
            }
        }
    }
}

/// Advance every `Watch` on item `j` that counts `what`, and run the payload of
/// any that came round.
///
/// A watcher never observes itself. Reactions already work that way and one
/// rule is easier to hold than two - and a fast item watching its own
/// activations would be counting its cadence, which is what `OnActivate` is
/// for.
fn tick_watchers(
    p: &mut Combatant,
    foes: &mut Vec<Combatant>,
    me: Ref,
    j: usize,
    what: Watched,
    t: u32,
    log: &mut Vec<LogEntry>,
) {
    let due: Vec<(Action, String)> = {
        let it = &mut pick(p, foes, me).items[j];
        let mut due = Vec::new();
        for k in 0..it.triggers.len() {
            let Trigger::Watch { what: w, count, then, repeats } = it.triggers[k] else { continue };
            if w != what || count == 0 || (!repeats && it.watch_paid[k]) {
                continue;
            }
            // The counter ticks after the event it watched has resolved, and
            // the payload runs immediately after that.
            it.watched[k] += 1;
            if it.watched[k] % count == 0 {
                it.watch_paid[k] = true;
                due.push((then, it.name.clone()));
            }
        }
        due
    };
    let front = aim_of(foes, p.aim);
    let (side, who) = (me.side, me.logged_as(front));
    for (action, item) in due {
        log.push(LogEntry {
            who,
            at_ms: t,
            event: Event::Watched { side, item, what: what.name() },
        });
        apply(p, foes, me, action, t, log, Some(j));
    }
}

/// Let every watcher on `side` count a curse landing.
///
/// Curses are the one thing a watcher counts that is not an activation, and
/// they land on both sides, so this is called from where the curse lands rather
/// than from where an item fires.
fn notify_curse_watchers(
    p: &mut Combatant,
    foes: &mut Vec<Combatant>,
    me: Ref,
    t: u32,
    log: &mut Vec<LogEntry>,
) {
    let count = pick(p, foes, me).items.len();
    for j in 0..count {
        tick_watchers(p, foes, me, j, Watched::CurseApplied, t, log);
    }
}

/// `owner` is the item the action belongs to, needed by effects that act on
/// the item itself rather than on a combatant.
fn apply(
    p: &mut Combatant,
    foes: &mut Vec<Combatant>,
    me: Ref,
    action: Action,
    t: u32,
    log: &mut Vec<LogEntry>,
    owner: Option<usize>,
) {
    let front = aim_of(foes, p.aim);
    let side = me.side;
    // Taken before any local rebinding shadows `me` with a combatant.
    let who = me.logged_as(front);
    // `Target::Yourself` means the side that owns the item, not the item's
    // victim — several strong items pay for themselves this way.
    let resolve = |target: Target| match target {
        Target::Enemy => me.other(front),
        Target::Yourself => me,
    };

    match action {
        Action::Fuse { a, b, into } => {
            // Both parents have to have something in them, and neither may
            // itself be a fusion - a product is not fuel. Anything else is a
            // no-op rather than a partial trade, so a board that fuses on a
            // fast cadence simply does nothing until it can afford to.
            let ok = !a.is_fused() && !b.is_fused() && into.is_fused() && a != b;
            let me_c = pick(p, foes, me);
            if ok && me_c.pool(a) > 0 && me_c.pool(b) > 0 {
                let (pa, pb) = (me_c.pool(a) - 1, me_c.pool(b) - 1);
                me_c.set_pool(a, pa);
                me_c.set_pool(b, pb);
                let total = me_c.pool(into) + 1;
                me_c.set_pool(into, total);
                log.push(LogEntry {
                    who,
                    at_ms: t,
                    event: Event::Fused {
                        side,
                        what: into.name(),
                        total,
                        from: (a.name(), pa),
                        and: (b.name(), pb),
                    },
                });
            }
        }
        Action::Curse { kind, target } => {
            // Bloodscent: what you rot, you feed on.
            if matches!(target, Target::Enemy) {
                let gain = pick(p, foes, me).bloodscent;
                if gain > 0 {
                    let me = pick(p, foes, me);
                    let total = me.pool(Resource::Rage) + gain;
                    me.set_pool(Resource::Rage, total);
                    log.push(LogEntry {
                        who,
                        at_ms: t,
                        event: Event::GainResource {
                            side,
                            what: Resource::Rage.name(),
                            amount: gain,
                            total,
                        },
                    });
                }
            }
            // Contagion: landing one brings the other along.
            let spread = if matches!(target, Target::Enemy) {
                pick(p, foes, me).contagion
            } else {
                0
            };
            for _ in 0..spread {
                // Contagion pairs a curse with its opposite number: heat and
                // cold, stopped and unreliable.
                let other = match kind {
                    CurseKind::Searing => CurseKind::Frost,
                    CurseKind::Frost => CurseKind::Searing,
                    CurseKind::Stun => CurseKind::Misfire,
                    CurseKind::Misfire => CurseKind::Stun,
                };
                let victim = pick(p, foes, me.other(front));
                land_curse(victim, me.other(front), other, StunAim::Unaimed, t, log);
            }
            let on = resolve(target);
            let c = pick(p, foes, on);
            land_curse(c, on, kind, StunAim::Unaimed, t, log);
            // A curse is the one thing a watcher counts that nobody activated,
            // and it is watched from both sides: the gear that landed it and
            // the gear wearing it both saw the same event.
            notify_curse_watchers(p, foes, me, t, log);
            notify_curse_watchers(p, foes, me.other(front), t, log);
        }
        Action::StunStrongest { target } => {
            let on = resolve(target);
            let c = pick(p, foes, on);
            land_curse(c, on, CurseKind::Stun, StunAim::Strongest, t, log);
            notify_curse_watchers(p, foes, me, t, log);
            notify_curse_watchers(p, foes, me.other(front), t, log);
        }
        Action::Damage { amount, kind, target } => {
            let on = resolve(target);
            // Next swing goes to the next one along. Done before the hit
            // resolves so a payload that lands twice still spreads.
            if me.side == Side::Player && on.side == Side::Enemy && foes.len() > 1 {
                p.aim = front + 1;
            }
            let pierce = match kind {
                DamageType::Physical => pick(p, foes, on.other(front)).physical_pierce,
                DamageType::Magic => pick(p, foes, on.other(front)).magic_pierce,
            };
            let c = pick(p, foes, on);
            let (absorbed, _) = c.take_typed(amount, kind, pierce);
            let (hp, ar) = (c.health, c.armor);
            log.push(LogEntry {
                who,
                at_ms: t,
                event: Event::Hit {
                    by: on.other(front).side,
                    damage: amount,
                    absorbed,
                    target_health: hp,
                    target_armor: ar,
                },
            });
        }
        Action::MindDamage { amount, target } => {
            let on = resolve(target);
            let c = pick(p, foes, on);
            let dealt = c.take_mind(amount);
            let mh = c.max_health;
            if dealt > 0 {
                log.push(LogEntry {
                    who,
                    at_ms: t,
                    event: Event::MindHit { by: on.other(front).side, amount: dealt, target_max_health: mh },
                });
            }
        }
        Action::Gain { what, amount } => {
            let me = pick(p, foes, me);
            let now = me.pool(what) + amount;
            me.set_pool(what, now);
            log.push(LogEntry {
                who,
                at_ms: t,
                event: Event::GainResource { side, what: what.name(), amount, total: now },
            });
        }
        Action::Drain { what, amount, hurt, target } => {
            let on = resolve(target);
            let c = pick(p, foes, on);
            let have = c.pool(what).max(0);
            // Zero means the lot. Taking more than they hold is not a debt -
            // an empty pool is simply empty.
            let taken = if amount == 0 { have } else { amount.min(have) };
            if taken > 0 {
                let left = have - taken;
                c.set_pool(what, left);
                log.push(LogEntry {
                    who,
                    at_ms: t,
                    event: Event::Drained {
                            on: on.side,
                            what: what.name(),
                            amount: taken,
                            total: left,
                        },
                });
                if hurt > 0 {
                    // Priced off what was actually taken, so a dry pool costs
                    // them nothing and a deep one costs them dearly.
                    let raw = taken * hurt;
                    let pierce = pick(p, foes, on.other(front)).magic_pierce;
                    let c = pick(p, foes, on);
                    let (absorbed, _) = c.take_typed(raw, DamageType::Magic, pierce);
                    let (hp, ar) = (c.health, c.armor);
                    log.push(LogEntry {
                        who,
                        at_ms: t,
                        event: Event::Hit {
                            by: on.other(front).side,
                            damage: raw,
                            absorbed,
                            target_health: hp,
                            target_armor: ar,
                        },
                    });
                }
            }
        }
        Action::GainMana(n) => {
            let c = pick(p, foes, me);
            c.mana += n;
            let total = c.mana;
            log.push(LogEntry { who: me.logged_as(front), at_ms: t, event: Event::GainMana { side, amount: n, total } });
        }
        Action::Grow(n) => {
            // Maximum health up, and the new room filled - growing into a gap
            // you then have to heal would make it useless in the fight that is
            // actually happening.
            let c = pick(p, foes, me);
            c.max_health += n;
            c.health += n;
            let total = c.max_health;
            log.push(LogEntry { who: me.logged_as(front), at_ms: t, event: Event::Grew { side, amount: n, total } });
        }
        Action::GainArmor(n) => {
            let c = pick(p, foes, me);
            // Consecrate: faith held makes the wall worth more. Gated on
            // actually holding some, so it rewards banking rather than being a
            // flat bonus wearing a name.
            let n = if c.consecrate > 0 && c.pool(Resource::Faith) > 0 {
                n + n * c.consecrate / 100
            } else {
                n
            };
            let n = c.gain_armor(n);
            let total = c.armor;
            log.push(LogEntry { who: me.logged_as(front), at_ms: t, event: Event::GainArmor { side, amount: n, total } });
        }
        Action::GainEmpowerment(n) => {
            let c = pick(p, foes, me);
            c.empowerment += n;
            let (total, bonus) = (c.empowerment, c.effective_power() - c.power);
            log.push(LogEntry {
                who,
                at_ms: t,
                event: Event::Empowered { side, total, power_bonus: bonus },
            });
        }
        Action::GainShield(n) => {
            let c = pick(p, foes, me);
            c.shield += n;
            let (total, reduction) = (c.shield, c.damage_reduction());
            log.push(LogEntry { who: me.logged_as(front), at_ms: t, event: Event::Shielded { side, total, reduction } });
        }
        Action::GainForking(n) => {
            let c = pick(p, foes, me);
            c.forking += n;
            let total = c.forking;
            log.push(LogEntry { who: me.logged_as(front), at_ms: t, event: Event::Forking { side, total } });
        }
        Action::ReduceCooldown(ms) => {
            let Some(idx) = owner else { return };
            let c = pick(p, foes, me);
            let Some(it) = c.items.get_mut(idx) else { return };
            // Push the bar forward rather than shortening the cooldown, so the
            // effect is "fires sooner once" and cannot stack into a free item.
            it.progress_ms = (it.progress_ms + ms).min(it.cooldown_ms.saturating_sub(1));
            let name = it.name.clone();
            log.push(LogEntry { who: me.logged_as(front), at_ms: t, event: Event::Hastened { side, item: name, by_ms: ms } });
        }
    }
}

// ---------------------------------------------------------------------------
// Alternates: creatures that are not on the ladder.
//
// An alternate stands in for a rung rather than adding one, so choosing to
// fight it does not lengthen the road. The ladder stays fifty long whichever
// way you go.

/// Creatures an event can put in front of you instead of the rung's own.
pub const ALTERNATES: &[MonsterSpec] = &[
    // The thing Nibbalonius will one day swallow, met early and still whole.
    // Armoured to start, regrows what it loses, and does no harm you can heal:
    // every point it takes off you it takes off your maximum.
    MonsterSpec {
        name: "The Dreaming Idiot",
        health: 520,
        strength: 0,
        regen: 2,
        mind_resist: 40,
        physical_resist: 22,
        magic_resist: 22,
        curse_resist: 45,
        attacks: &[],
        gear: &[
            ("Covenant Frame", SlotKind::Helmet, 0, 0, 0),
            ("Warded Plating", SlotKind::Helmet, 3, 0, 0),
            ("Covenant Frame", SlotKind::Helmet, 0, 2, 0),
            ("Bulwark Plating", SlotKind::Helmet, 3, 2, 0),
            ("Covenant Frame", SlotKind::Helmet, 0, 4, 0),
            ("Braced Plating", SlotKind::Helmet, 3, 4, 0),
            ("Hexweave Shroud", SlotKind::Chest, 0, 0, 0),
            ("Seedbed Layer", SlotKind::Chest, 3, 0, 0),
            ("Seedbed Layer", SlotKind::Chest, 3, 1, 0),
            ("Deep Roots Base", SlotKind::Chest, 3, 2, 0),
            ("Seedbed Layer", SlotKind::Chest, 0, 3, 0),
            ("Hexweave Shroud", SlotKind::Chest, 0, 4, 0),
            ("Seedbed Layer", SlotKind::Chest, 3, 4, 0),
            ("Duskweave Material", SlotKind::Gloves, 0, 0, 0),
            ("Empowering Mold", SlotKind::Gloves, 3, 0, 0),
            ("Tithe Ring", SlotKind::Gloves, 4, 0, 0),
            ("Duskweave Material", SlotKind::Gloves, 0, 2, 0),
            ("Empowering Mold", SlotKind::Gloves, 3, 2, 0),
            ("Duskweave Material", SlotKind::Gloves, 0, 4, 0),
            ("Channeling Mold", SlotKind::Gloves, 3, 4, 0),
            ("Ring of Tides", SlotKind::Gloves, 0, 3, 0),
            ("Tithe Ring", SlotKind::Gloves, 2, 3, 0),
            ("Duskweave Material", SlotKind::Greaves, 0, 0, 0),
            ("Standing Start", SlotKind::Greaves, 3, 0, 0),
            ("Mana Ward", SlotKind::Greaves, 2, 1, 0),
            ("Duskweave Material", SlotKind::Greaves, 0, 2, 0),
            ("Striding Mold", SlotKind::Greaves, 2, 3, 0),
            ("Braced Plating", SlotKind::Greaves, 4, 2, 0),
            ("Duskweave Material", SlotKind::Greaves, 0, 4, 0),
            ("Striding Mold", SlotKind::Greaves, 2, 5, 0),
            // One voice. A creature that deals nothing but mind damage has
            // exactly one weapon in it: the orb-and-Unmaking build is the whole
            // of what the catalogue offers that does no other kind of harm.
            ("Grovemind Orb", SlotKind::Weapon, 0, 0, 0),
            ("Siphon", SlotKind::Weapon, 3, 0, 0),
            ("Siphon", SlotKind::Weapon, 4, 0, 0),
            ("Siphon", SlotKind::Weapon, 5, 0, 0),
            ("Rootwork Alignment", SlotKind::Weapon, 0, 2, 0),
        ],
        gear_offset: 0,
        bounty: 140,
        sprite: MonsterSprite::Idiot,
        rank: Rank::Boss,
        drops: &["The Idiot's Gift"],
        items: &[2, 2, 2, 3, 2, 2, 3, 2, 4, 3, 3, 2, 5],
    },
    // ---- Bunko's Cavern, pp. 84-85 ------------------------------------------
    //
    // Floor one: the Head Cork Priest of Corrqk's Cavern, reciting the '62
    // Anticipations to a room of workers kneeling on a floor that cuts.
    MonsterSpec {
        name: "The Reciter",
        health: 430,
        strength: 14,
        regen: 2,
        mind_resist: 25,
        physical_resist: 14,
        magic_resist: 20,
        curse_resist: 30,
        attacks: &[],
        gear: &[
            ("Covenant Frame", SlotKind::Helmet, 0, 0, 0),
            ("Braced Plating", SlotKind::Helmet, 3, 0, 0),
            ("Covenant Frame", SlotKind::Helmet, 0, 2, 0),
            ("Mana Ward", SlotKind::Helmet, 3, 2, 0),
            ("Vigil Crest", SlotKind::Helmet, 5, 0, 0),
            ("Bloodbank Base", SlotKind::Chest, 0, 0, 0),
            ("Split Weave", SlotKind::Chest, 2, 0, 0),
            ("Hexweave Shroud", SlotKind::Chest, 2, 1, 0),
            ("Seedbed Layer", SlotKind::Chest, 0, 4, 0),
            ("Duskweave Material", SlotKind::Gloves, 0, 0, 0),
            ("Empowering Mold", SlotKind::Gloves, 3, 0, 0),
            ("Ring of Tides", SlotKind::Gloves, 4, 0, 0),
            ("Ring of Tides", SlotKind::Gloves, 5, 0, 0),
            ("Warmed Material", SlotKind::Gloves, 0, 2, 0),
            ("Empowering Mold", SlotKind::Gloves, 2, 1, 0),
            ("Ring of Tides", SlotKind::Gloves, 0, 1, 0),
            ("Ring of Tides", SlotKind::Gloves, 4, 2, 0),
            ("Duskweave Material", SlotKind::Greaves, 0, 0, 0),
            ("Standing Start", SlotKind::Greaves, 3, 0, 0),
            ("Warmed Material", SlotKind::Greaves, 2, 1, 0),
            ("Striding Mold", SlotKind::Greaves, 4, 1, 0),
            ("Braced Plating", SlotKind::Greaves, 0, 2, 0),
            ("Ambusher's Grip", SlotKind::Weapon, 0, 0, 0),
            ("Cursed Blade", SlotKind::Weapon, 1, 0, 0),
            ("Empowering Focus", SlotKind::Weapon, 3, 0, 0),
            ("Empowering Focus", SlotKind::Weapon, 4, 1, 0),
            ("Ambusher's Grip", SlotKind::Weapon, 2, 2, 0),
            ("Cursed Blade", SlotKind::Weapon, 3, 2, 0),
            ("Cursed Blade", SlotKind::Weapon, 0, 3, 0),
            ("Bulwark Bead", SlotKind::Weapon, 1, 3, 0),
            ("Bulwark Bead", SlotKind::Weapon, 5, 3, 0),
        ],
        gear_offset: 0,
        bounty: 96,
        sprite: MonsterSprite::Abbot,
        rank: Rank::Mini,
        drops: &[],
        items: &[2, 3, 2, 2, 4, 4, 2, 3, 4, 5],
    },
    // Floor two: the train the dissenters were loaded onto, still running.
    MonsterSpec {
        name: "The Long Haul",
        health: 620,
        strength: 22,
        regen: 0,
        mind_resist: 10,
        physical_resist: 26,
        magic_resist: 10,
        curse_resist: 20,
        attacks: &[],
        gear: &[
            ("Covenant Frame", SlotKind::Helmet, 0, 0, 0),
            ("Mirrored Visor", SlotKind::Helmet, 3, 0, 0),
            ("Covenant Frame", SlotKind::Helmet, 0, 2, 0),
            ("Mana Ward", SlotKind::Helmet, 3, 2, 0),
            ("Third Eye", SlotKind::Helmet, 0, 1, 0),
            ("Hexweave Shroud", SlotKind::Chest, 0, 0, 0),
            ("Aether Layer", SlotKind::Chest, 3, 0, 0),
            ("Bloodbank Base", SlotKind::Chest, 3, 2, 0),
            ("Split Weave", SlotKind::Chest, 0, 3, 0),
            ("Duskweave Material", SlotKind::Gloves, 0, 0, 0),
            ("Empowering Mold", SlotKind::Gloves, 3, 0, 0),
            ("Ring of Tides", SlotKind::Gloves, 4, 0, 0),
            ("Ring of Tides", SlotKind::Gloves, 5, 0, 0),
            ("Warmed Material", SlotKind::Gloves, 0, 2, 0),
            ("Rending Mold", SlotKind::Gloves, 2, 2, 0),
            ("Ring of Tides", SlotKind::Gloves, 0, 1, 0),
            ("Ironthread Material", SlotKind::Greaves, 0, 0, 0),
            ("Standing Start", SlotKind::Greaves, 3, 0, 0),
            ("Duskweave Material", SlotKind::Greaves, 2, 1, 0),
            ("Standing Start", SlotKind::Greaves, 0, 2, 0),
            ("Ambusher's Grip", SlotKind::Weapon, 0, 0, 0),
            ("Cursed Blade", SlotKind::Weapon, 1, 0, 0),
            ("Cursed Blade", SlotKind::Weapon, 3, 0, 0),
            ("Bulwark Bead", SlotKind::Weapon, 2, 0, 0),
            ("Ambusher's Grip", SlotKind::Weapon, 5, 0, 0),
            ("Cursed Blade", SlotKind::Weapon, 4, 2, 0),
            ("Grimoire Rack", SlotKind::Weapon, 3, 3, 0),
        ],
        gear_offset: 0,
        bounty: 104,
        sprite: MonsterSprite::Parliament,
        rank: Rank::Mini,
        drops: &[],
        items: &[2, 3, 2, 2, 4, 3, 2, 2, 4, 3],
    },
    // Floor three: the old gods, watching in horror as he ascends.
    MonsterSpec {
        name: "The Watchers",
        health: 880,
        strength: 20,
        regen: 6,
        mind_resist: 45,
        physical_resist: 24,
        magic_resist: 34,
        curse_resist: 45,
        attacks: &[],
        gear: &[
            ("Covenant Frame", SlotKind::Helmet, 0, 0, 0),
            ("Broken Crown", SlotKind::Helmet, 0, 1, 0),
            ("Third Eye", SlotKind::Helmet, 3, 0, 0),
            ("Covenant Frame", SlotKind::Helmet, 1, 3, 0),
            ("Braced Plating", SlotKind::Helmet, 0, 4, 0),
            ("Third Eye", SlotKind::Helmet, 3, 4, 0),
            ("Covenant Frame", SlotKind::Helmet, 2, 5, 0),
            ("Mana Ward", SlotKind::Helmet, 0, 6, 0),
            ("Mana Ward", SlotKind::Helmet, 4, 3, 1),
            ("Bloodbank Base", SlotKind::Chest, 0, 0, 0),
            ("Seedbed Layer", SlotKind::Chest, 2, 0, 0),
            ("Bloodbank Base", SlotKind::Chest, 2, 1, 0),
            ("Seedbed Layer", SlotKind::Chest, 0, 3, 0),
            ("Bloodbank Base", SlotKind::Chest, 4, 1, 0),
            ("Seedbed Layer", SlotKind::Chest, 3, 3, 0),
            ("Aether Layer", SlotKind::Chest, 2, 4, 0),
            ("Warmed Material", SlotKind::Gloves, 0, 0, 0),
            ("Empowering Mold", SlotKind::Gloves, 2, 0, 0),
            ("Seal of the Deep", SlotKind::Gloves, 3, 0, 0),
            ("Warmed Material", SlotKind::Gloves, 4, 1, 0),
            ("Hexer's Reckoning", SlotKind::Gloves, 2, 2, 0),
            ("Warmed Material", SlotKind::Gloves, 0, 2, 0),
            ("Empowering Mold", SlotKind::Gloves, 0, 4, 0),
            ("Ring of Tides", SlotKind::Gloves, 1, 4, 0),
            ("Warding Ring", SlotKind::Gloves, 2, 4, 0),
            ("Ironthread Material", SlotKind::Greaves, 0, 0, 0),
            ("Striding Mold", SlotKind::Greaves, 3, 0, 0),
            ("Ironthread Material", SlotKind::Greaves, 0, 2, 0),
            ("Striding Mold", SlotKind::Greaves, 2, 1, 0),
            ("Warmed Material", SlotKind::Greaves, 4, 2, 0),
            ("Standing Start", SlotKind::Greaves, 2, 4, 0),
            ("Broken Crown", SlotKind::Greaves, 0, 5, 0),
            ("Balanced Grip", SlotKind::Weapon, 0, 0, 0),
            ("Cursed Blade", SlotKind::Weapon, 1, 0, 0),
            ("Whetstone", SlotKind::Weapon, 2, 0, 0),
            ("Gravebound Haft", SlotKind::Weapon, 3, 0, 0),
            ("Cursed Blade", SlotKind::Weapon, 3, 2, 0),
            ("Balanced Grip", SlotKind::Weapon, 5, 0, 0),
            ("Cursed Blade", SlotKind::Weapon, 4, 4, 2),
            ("Balance Weight", SlotKind::Weapon, 2, 5, 0),
        ],
        gear_offset: 0,
        bounty: 150,
        sprite: MonsterSprite::Choir,
        rank: Rank::Boss,
        drops: &["The Split Wisdom"],
        items: &[3, 3, 3, 2, 2, 3, 3, 2, 4, 2, 2, 3, 3, 2, 3],
    },
];

/// The floors of Bunko's Cavern, pp. 84-85. Authored by the packing tool like
/// every other named board; the gear lists are pasted from its output.
pub const CREVICE: &[MonsterSpec] = &[];

/// An alternate by name.
pub fn alternate(name: &str) -> Option<&'static MonsterSpec> {
    ALTERNATES.iter().find(|m| m.name == name)
}

/// Any creature in the game by name, wherever it is written.
///
/// `alternate` only knows the ones written specially for events. An event that
/// wants two creatures off the ladder itself - a pair of gamblers who are also
/// rungs twelve and thirteen - needs to find those too.
pub fn creature(name: &str) -> Option<&'static MonsterSpec> {
    LADDER.iter().find(|m| m.name == name).or_else(|| alternate(name))
}


#[cfg(test)]
mod stun_aim_tests {
    use super::*;
    use crate::stats::Stats;

    /// A fighter carrying items that differ only in how good they are.
    fn victim(ratings: &[i32]) -> Combatant {
        let mut c = Combatant::player(Stats::ZERO, &[]);
        c.items = ratings
            .iter()
            .enumerate()
            .map(|(i, &rating)| RunningItem {
                name: format!("item {i}"),
                rating,
                cooldown_ms: 1000,
                ..Default::default()
            })
            .collect();
        c
    }

    #[test]
    fn an_aimed_stun_always_takes_the_best_item() {
        let mut c = victim(&[10, 90, 40, 5]);
        for t in [0, 700, 1500, 2600] {
            let (idx, _) = land_stun(&mut c, StunAim::Strongest, t).expect("a stun landed");
            assert_eq!(idx, 1, "aimed at t={t} and missed the 90-rated item");
        }
    }

    #[test]
    fn an_unaimed_stun_spreads_across_the_kit() {
        let mut c = victim(&[10, 90, 40, 5]);
        let mut seen: Vec<usize> = Vec::new();
        // Four stuns, and nothing is stopped for long enough to still be
        // stopped when the next one lands.
        for (n, t) in [0u32, 5_000, 10_000, 15_000].into_iter().enumerate() {
            for item in &mut c.items {
                item.stun_ms = 0;
            }
            let (idx, _) = land_stun(&mut c, StunAim::Unaimed, t).expect("a stun landed");
            assert!(idx < 4, "picked item {idx} of four on stun {n}");
            if !seen.contains(&idx) {
                seen.push(idx);
            }
        }
        assert!(
            seen.len() >= 2,
            "four unaimed stuns all landed on item {seen:?} - it is meant to pick without \
             warning, not to be predictable"
        );
    }

    #[test]
    fn an_unaimed_stun_prefers_an_item_that_is_still_running() {
        let mut c = victim(&[10, 20, 30]);
        c.items[0].stun_ms = 900;
        c.items[2].stun_ms = 900;
        // Only item 1 is live, so wherever the hash points it has to end there
        // - burying an already-stopped item is the one thing this must not do.
        for t in [0, 350, 900, 1250, 4000] {
            let (idx, _) = land_stun(&mut c, StunAim::Unaimed, t).expect("a stun landed");
            assert_eq!(idx, 1, "at t={t} it stunned something already stopped");
            c.items[1].stun_ms = 0;
        }
    }

    #[test]
    fn stacking_piles_onto_one_clock_and_stops_at_the_cap() {
        let mut c = victim(&[10, 90]);
        let base = CurseKind::Stun.landing_ms(0);
        let (_, first) = land_stun(&mut c, StunAim::Strongest, 0).unwrap();
        assert_eq!(first, base);
        let (_, second) = land_stun(&mut c, StunAim::Strongest, 100).unwrap();
        assert_eq!(second, base * 2, "a second stun on the same item has to add to the clock");
        for t in 0..20 {
            land_stun(&mut c, StunAim::Strongest, t * 100);
        }
        assert_eq!(c.items[1].stun_ms, STUN_CAP_MS, "a stun chain is not a lock");
        assert_eq!(c.items[0].stun_ms, 0, "the aimed stun never wandered off its target");
    }

    #[test]
    fn a_fully_resistant_target_is_never_stunned() {
        let mut c = victim(&[10, 90]);
        c.curse_resist = 100;
        assert!(land_stun(&mut c, StunAim::Strongest, 0).is_none());
        assert!(land_stun(&mut c, StunAim::Unaimed, 0).is_none());
        assert!(c.items.iter().all(|i| i.stun_ms == 0));
    }

    #[test]
    fn a_fighter_with_no_items_cannot_be_stunned() {
        let mut c = victim(&[]);
        assert!(land_stun(&mut c, StunAim::Unaimed, 0).is_none());
    }
}
