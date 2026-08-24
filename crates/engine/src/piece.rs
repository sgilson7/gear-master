use crate::combat::DamageType;
use crate::curse::CurseKind;
use crate::shape::Shape;
use crate::stats::{StatKind, Stats};

/// The five equipment slots. Each is its own 6x8 grid.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SlotKind {
    Helmet,
    Chest,
    Gloves,
    Greaves,
    Weapon,
}

impl SlotKind {
    pub const ALL: [SlotKind; 5] = [
        SlotKind::Helmet,
        SlotKind::Chest,
        SlotKind::Gloves,
        SlotKind::Greaves,
        SlotKind::Weapon,
    ];

    pub fn index(self) -> usize {
        match self {
            SlotKind::Helmet => 0,
            SlotKind::Chest => 1,
            SlotKind::Gloves => 2,
            SlotKind::Greaves => 3,
            SlotKind::Weapon => 4,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SlotKind::Helmet => "Helmet",
            SlotKind::Chest => "Chestpiece",
            SlotKind::Gloves => "Gloves",
            SlotKind::Greaves => "Greaves",
            SlotKind::Weapon => "Weapon",
        }
    }

    /// What a valid assembly in this slot needs, in one line.
    ///
    /// Built from the recipe table rather than written out, so it cannot drift
    /// from the rule it describes. The interface uses `recipe_parts` instead,
    /// which keeps the required and the optional halves separate.
    pub fn recipe_text(self) -> String {
        recipe_parts(self)
            .iter()
            .map(|p| {
                let mut s = p.required.join(" + ");
                if !p.optional.is_empty() {
                    s.push_str(", plus up to ");
                    s.push_str(&p.optional.join(" and "));
                }
                s
            })
            .collect::<Vec<_>>()
            .join(", OR ")
    }
}

/// One way of building a slot, split at the line that matters: what an item
/// must have before it counts as assembled, and what may be added on top.
///
/// Everything in `optional` is an improvement to gear that already works. A
/// helmet is finished with a frame and one plating; the second plating and the
/// crest make it better, not valid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecipeParts {
    /// What to call this way of building, when a slot offers several.
    pub title: &'static str,
    /// The minimum that makes an item, e.g. `["1 frame", "1 plating"]`.
    pub required: Vec<String>,
    /// What may be added beyond that, e.g. `["1 more plating", "1 crest"]`.
    pub optional: Vec<String>,
}

/// `(2, PieceKind::Accessory)` -> `"accessories"`.
fn noun(n: usize, kind: PieceKind, slot: SlotKind) -> String {
    let name = kind.name_in(slot);
    if n == 1 {
        return name;
    }
    match kind {
        // Mass nouns: "2 platings" is not English.
        PieceKind::Damaging | PieceKind::Plating => name,
        _ if name.ends_with('y') => format!("{}ies", &name[..name.len() - 1]),
        _ => format!("{}s", name),
    }
}

/// `(2, PieceKind::Accessory)` -> `"2 accessories"`.
fn count_of(n: usize, kind: PieceKind, slot: SlotKind) -> String {
    format!("{} {}", n, noun(n, kind, slot))
}

/// Every way of building `slot`, each split into required and optional.
pub fn recipe_parts(slot: SlotKind) -> Vec<RecipeParts> {
    recipes(slot)
        .iter()
        .map(|r| {
            let mut required = Vec::new();
            let mut optional = Vec::new();
            for &(kind, min, max) in *r {
                if min > 0 {
                    required.push(count_of(min, kind, slot));
                }
                if max > min {
                    // "1 more plating" when some was already required, so it
                    // is clear this is on top of the minimum rather than an
                    // alternative to it.
                    let n = max - min;
                    optional.push(if min > 0 {
                        format!("{} more {}", n, noun(n, kind, slot))
                    } else {
                        count_of(n, kind, slot)
                    });
                }
            }
            // Named for the piece it is built around, which is the thing that
            // decides which recipe you are following.
            let title = r
                .iter()
                .find(|(k, ..)| k.is_core())
                .map(|(k, ..)| match k {
                    PieceKind::Handle => "Martial weapon",
                    PieceKind::Book => "Book spell",
                    PieceKind::Orb => "Crystal ball",
                    _ => "",
                })
                .unwrap_or("");
            RecipeParts { title, required, optional }
        })
        .collect()
}

/// What role a component plays inside its slot's recipe. Which slot a given
/// piece belongs to is declared on the `PieceDef` itself, because gloves and
/// greaves both build from materials and molds.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum PieceKind {
    // Weapon
    Handle,
    Damaging,
    Accessory,
    // Helmet
    Frame,
    Plating,
    Crest,
    // Chest
    Base,
    Layer,
    // Gloves + greaves
    Material,
    Mold,
    /// Worn on the hands. Up to two to a glove.
    Ring,
    // Weapon, the arcane way: a book or an orb sets the cadence, ink scales
    // the payload, and the spell is the payload.
    Book,
    Ink,
    Spell,
    Orb,
    /// Set into a crystal ball. It colours every spell the ball holds rather
    /// than being cast itself - which is why an orb needs no ink: the
    /// alignment is where an orb's build decision lives.
    Alignment,
    /// **Terrain.** Laid under the grid rather than packed into it: other
    /// pieces may sit on top of it, and what it is worth depends on what ends
    /// up covering it.
    ///
    /// No recipe names this kind, which is the whole of how "an underlay is
    /// never part of an item" is enforced - there is no rule to write and no
    /// special case to forget. It is a kind rather than a flag on `PieceDef`
    /// because terrain is a different sort of thing from gear, not gear with a
    /// setting; the spec called for a `bool`, and a bool would also have meant
    /// spelling out `underlay: false` in all 446 existing entries.
    Terrain,
}

impl PieceKind {
    /// The component each recipe needs exactly one of. A core anchors an item:
    /// everything else in the slot joins the core it is nearest to, which is
    /// what lets two finished items sit flush against each other.
    /// Does this kind lie under the grid rather than in it?
    pub fn is_underlay(self) -> bool {
        matches!(self, PieceKind::Terrain)
    }

    pub fn is_core(self) -> bool {
        matches!(
            self,
            PieceKind::Handle
                | PieceKind::Frame
                | PieceKind::Base
                | PieceKind::Material
                | PieceKind::Book
                | PieceKind::Orb
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            PieceKind::Terrain => "terrain",
            PieceKind::Ring => "ring",
            PieceKind::Book => "book",
            PieceKind::Ink => "ink",
            PieceKind::Spell => "spell",
            PieceKind::Orb => "crystal ball",
            PieceKind::Handle => "handle",
            PieceKind::Damaging => "damaging",
            PieceKind::Accessory => "accessory",
            PieceKind::Frame => "frame",
            PieceKind::Plating => "plating",
            PieceKind::Crest => "crest",
            PieceKind::Base => "base",
            PieceKind::Layer => "layer",
            PieceKind::Material => "material",
            PieceKind::Mold => "mold",
            PieceKind::Alignment => "alignment",
        }
    }

    /// The name to show when the piece is known to belong to `slot`.
    ///
    /// Two slots can call for the same role without the pieces being
    /// interchangeable: gloves and greaves both want a mold, but a glove's
    /// mold will not go on a shin. Calling both "mold" invites exactly the
    /// wrong conclusion, so a role used by several slots is qualified by its
    /// slot - "gloves mold" - unless its pieces really are shared, in which
    /// case the bare name is the honest one.
    pub fn name_in(self, slot: SlotKind) -> String {
        if self.is_slot_specific() {
            format!("{} {}", slot.name().to_lowercase(), self.name())
        } else {
            self.name().to_string()
        }
    }

    /// Is this a role that several slots want, but whose pieces do not carry
    /// between them? Derived rather than listed, so new gear cannot quietly
    /// reintroduce the ambiguity.
    pub fn is_slot_specific(self) -> bool {
        let shareable = CATALOG.iter().any(|d| d.kind == self && d.shared());
        if shareable {
            return false;
        }
        SlotKind::ALL
            .iter()
            .filter(|&&s| recipes(s).iter().any(|r| r.iter().any(|(k, ..)| *k == self)))
            .count()
            > 1
    }
}

/// A flat stat bonus that fires **only** once the piece's item assembles into
/// finished gear — Gear Master's version of a Backpack Battles adjacency
/// bonus. Exactly one piece per slot carries one.
#[derive(Copy, Clone, Debug)]
pub struct Adjacency {
    pub label: &'static str,
    pub stats: Stats,
}

/// When a piece's `Effect` is live, relative to whether the item it is part of
/// came together. `NotAssembled` is the deliberate inverse: gear that is worth
/// more left in pieces than finished.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum When {
    Always,
    Assembled,
    NotAssembled,
}

impl When {
    pub fn holds(self, assembled: bool) -> bool {
        match self {
            When::Always => true,
            When::Assembled => assembled,
            When::NotAssembled => !assembled,
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            When::Always => "",
            When::Assembled => " (while assembled)",
            When::NotAssembled => " (while NOT assembled)",
        }
    }
}

/// What a piece does to — or because of — its surroundings, over and above its
/// flat `base` stats.
#[derive(Copy, Clone, Debug)]
pub enum EffectKind {
    /// Every orthogonally adjacent piece of `kind` contributes double its
    /// `stat`. Applied at most once per neighbour, however many sources touch
    /// it.
    DoubleNeighbor { kind: PieceKind, stat: StatKind },
    /// This piece itself gains `per` of `stat` for every in-bounds empty cell
    /// orthogonally touching its own footprint.
    SelfPerEmptyCell { stat: StatKind, per: i32 },
    /// Flat stats, gated by the effect's `when`. With `When::NotAssembled`
    /// this is how a piece can be worth more left in bits than built up.
    Flat { stats: Stats },
    /// Every OTHER assembled item touching this piece contributes double its
    /// `stat`. Cross-item, which is only expressible because items are anchored
    /// by their core and may therefore sit flush against one another.
    DoubleAdjacentItemStat { stat: StatKind },
    /// Terrain only: `amount` of `stat` for every distinct piece covering at
    /// least one of this piece's cells.
    PerOverlappingItem { stat: StatKind, amount: i32 },
    /// Terrain only: the same, counting only the cores items are built around.
    ///
    /// Worth more than `PerOverlappingItem` and harder to earn - a core is one
    /// piece an item, so covering an underlay with two cores means two items
    /// standing on it rather than one item spread across it.
    PerOverlappingCore { stat: StatKind, amount: i32 },
    /// Multiply every number on this item by `times`, but only while the item
    /// is standing alone in the sense `what` describes.
    ///
    /// The point of a build being a set of five grids is that they are five
    /// grids; this is what makes *where* you put a thing matter as much as
    /// what it is. The multipliers are enormous and the conditions are very
    /// easy to break by accident, which is the trade.
    SoleIf { what: Solitude, times: i32 },
    /// This piece gains `per` of `stat` for every orthogonally adjacent piece
    /// of `kind` in the same grid. Where `DoubleNeighbor` reaches out and
    /// changes what its neighbours are worth, this reads them and changes
    /// what *it* is worth - so a piece can be built to reward being packed
    /// against a particular sort of thing.
    SelfPerNeighborKind { kind: PieceKind, stat: StatKind, per: i32 },
}

#[derive(Copy, Clone, Debug)]
pub struct Effect {
    pub label: &'static str,
    pub when: When,
    pub kind: EffectKind,
}

impl Effect {
    /// Full description including the condition, for tooltips and the CLI.
    pub fn describe(&self) -> String {
        format!("{}{}", self.label, self.when.suffix())
    }
}

/// What "standing alone" has to mean for a `SoleIf` multiplier to pay.
///
/// All of these look at *assembled* items only. Loose pieces are not gear and
/// do not crowd anything.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Solitude {
    /// No other finished item anywhere on the board occupies a row this one
    /// occupies. The strictest of the three, and worth the most.
    Row,
    /// Lay the five grids on top of one another: no other finished item covers
    /// a cell this one covers.
    Stacked,
    /// The same, but only items in this slot are counted.
    StackedWith(SlotKind),
}

impl Solitude {
    pub fn describe(self) -> String {
        match self {
            Solitude::Row => "no other finished item shares a row with it".into(),
            Solitude::Stacked => {
                "no other finished item overlaps it with the grids stacked".into()
            }
            Solitude::StackedWith(s) => format!(
                "no finished {} overlaps it with the grids stacked",
                s.name().to_lowercase()
            ),
        }
    }
}

/// Who an effect lands on. Items can curse their own wearer — several of the
/// stronger ones do exactly that as their cost.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Target {
    Enemy,
    Yourself,
}

impl Target {
    pub fn name(self) -> &'static str {
        match self {
            Target::Enemy => "the enemy",
            Target::Yourself => "yourself",
        }
    }
}

/// A banked pool.
///
/// The first four are what gear grants and spends. The last three are
/// **fusions**: made by spending one of each of two parents, worth both
/// parents' passive at double rate, and no use as fuel - nothing spends them,
/// which is what stops a fusion from being a second currency with better rates.
/// They can still be drained, so a build that banks deeply in one is carrying
/// something worth taking.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Resource {
    Mana,
    Rage,
    Faith,
    Nature,
    DruidicMight,
    Communion,
    Zealotry,
}

impl Resource {
    pub const ALL: [Resource; 7] = [
        Resource::Mana,
        Resource::Rage,
        Resource::Faith,
        Resource::Nature,
        Resource::DruidicMight,
        Resource::Communion,
        Resource::Zealotry,
    ];

    /// The four a trigger may spend. Mana is fuel and the other three are
    /// holdings, but all four are things gear can ask for; a fusion is not.
    pub const SPENDABLE: [Resource; 4] =
        [Resource::Mana, Resource::Rage, Resource::Faith, Resource::Nature];

    /// A stable slot for a per-resource array. `Run::banked_all_run` is
    /// indexed by it, so these numbers are not free to move.
    pub fn index(self) -> usize {
        match self {
            Resource::Mana => 0,
            Resource::Rage => 1,
            Resource::Faith => 2,
            Resource::Nature => 3,
            Resource::DruidicMight => 4,
            Resource::Communion => 5,
            Resource::Zealotry => 6,
        }
    }

    /// A product rather than fuel: `Spend`, `SpendMana` and `Consume` refuse
    /// it, and `Fuse` will not take one as a parent.
    pub fn is_fused(self) -> bool {
        matches!(self, Resource::DruidicMight | Resource::Communion | Resource::Zealotry)
    }

    /// The two pools a fusion is made of, in the order they are spent.
    pub fn parents(self) -> Option<(Resource, Resource)> {
        match self {
            Resource::DruidicMight => Some((Resource::Nature, Resource::Rage)),
            Resource::Communion => Some((Resource::Faith, Resource::Nature)),
            Resource::Zealotry => Some((Resource::Rage, Resource::Faith)),
            _ => None,
        }
    }

    /// The reverse of `name`. Combat logs a resource by its name, so anything
    /// reading a log back needs this.
    pub fn by_name(name: &str) -> Option<Resource> {
        Resource::ALL.into_iter().find(|r| r.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Resource::Mana => "mana",
            Resource::Rage => "rage",
            Resource::Faith => "faith",
            Resource::Nature => "nature",
            Resource::DruidicMight => "druidic might",
            Resource::Communion => "communion",
            Resource::Zealotry => "zealotry",
        }
    }
}

/// Something an item does at the moment it activates, beyond its flat stats.
#[derive(Copy, Clone, Debug)]
pub enum Action {
    Curse { kind: CurseKind, target: Target },
    /// A stun that picks its target: the best item the victim owns, by the
    /// same effectiveness rating the shop prices gear with.
    ///
    /// A plain curse of stun takes whatever item it catches, which is most of
    /// what keeps it fair. Choosing costs more than the stun does.
    StunStrongest { target: Target },
    Damage { amount: i32, kind: crate::combat::DamageType, target: Target },
    MindDamage { amount: i32, target: Target },
    GainMana(i32),
    /// Bank any of the four pools.
    Gain { what: Resource, amount: i32 },
    /// Take a pool off the target, and make the loss hurt.
    ///
    /// `amount` is how much to take, or 0 for the lot. `hurt` is magic damage
    /// dealt per point actually taken - so it is worth nothing against an
    /// empty pool and a great deal against a build that banks deeply. That
    /// asymmetry is the point: it punishes hoarding rather than punishing
    /// everyone equally.
    Drain { what: Resource, amount: i32, hurt: i32, target: Target },
    GainArmor(i32),
    /// Push this item's cooldown forward, so it fires sooner.
    ReduceCooldown(u32),
    /// Gain stacks of mana empowerment: each stack adds 0.05x weapon power per
    /// point of mana you are currently holding.
    GainEmpowerment(u32),
    /// Gain stacks of mana shield: each stack cuts 1 point off every incoming
    /// hit per point of mana you are holding, whatever the damage type.
    GainShield(u32),
    /// Gain stacks of spell forking: every cast lands once more per stack.
    ///
    /// Only a spell forks. A blade swings once however many stacks are up -
    /// which is what makes this the caster's answer to a build that out-swings
    /// it, rather than a flat damage buff wearing a different name.
    GainForking(u32),
    /// Raise maximum health by `n` for the rest of the fight, and heal for it.
    ///
    /// The only thing in the game that grows while a fight is running, so a
    /// piece carrying it is worth more the longer the fight lasts - which is
    /// the opposite of everything else, and why they cost what they cost.
    Grow(i32),
    /// Spend one of each parent pool to bank one of a fused pool. Nothing
    /// happens unless both parents have something in them.
    ///
    /// The exchange is deliberately bad by volume and good by rate: two points
    /// become one, and that one is worth both parents at double. So fusing is
    /// only worth it once income outruns what the passives are paying, which
    /// makes it a decision late in a fight rather than a thing to do on sight.
    Fuse { a: Resource, b: Resource, into: Resource },
}

impl Action {
    pub fn describe(&self) -> String {
        match self {
            Action::Curse { kind, target } => {
                format!("apply curse of {} to {}", kind.name(), target.name())
            }
            Action::Fuse { a, b, into } => {
                format!("turn 1 {} and 1 {} into 1 {}", a.name(), b.name(), into.name())
            }
            Action::StunStrongest { target } => {
                format!("stun the strongest item {} has", target.name())
            }
            Action::Damage { amount, kind, target } => {
                format!("deal {} {} to {}", amount, kind.name(), target.name())
            }
            Action::MindDamage { amount, target } => {
                format!("deal {} mind damage to {}", amount, target.name())
            }
            Action::GainMana(n) => format!("gain {} mana", n),
            Action::Gain { what, amount } => format!("gain {} {}", amount, what.name()),
            Action::Drain { what, amount, hurt, target } => {
                let take = if *amount == 0 {
                    format!("all of {}'s {}", target.name(), what.name())
                } else {
                    format!("{} {} from {}", amount, what.name(), target.name())
                };
                if *hurt > 0 {
                    format!("drain {} and deal {} magic for each point", take, hurt)
                } else {
                    format!("drain {}", take)
                }
            }
            Action::GainArmor(n) => format!("gain {} armor", n),
            Action::ReduceCooldown(ms) => {
                format!("cut {:.1}s off its own cooldown", *ms as f32 / 1000.0)
            }
            Action::GainEmpowerment(n) => format!("gain {} mana empowerment", n),
            Action::GainShield(n) => format!("gain {} mana shield", n),
            Action::GainForking(n) => format!("gain {} spell forking", n),
            Action::Grow(n) => format!("gain {} maximum health for the rest of the fight", n),
        }
    }
}

impl Action {
    /// This action with its numbers multiplied by `pct` hundredths.
    ///
    /// Outcomes only. What a trigger *costs* is never scaled: a piece that
    /// spends four mana spends four mana whatever multiplier the item is
    /// carrying, or power would quietly price a build out of its own gear.
    pub fn scaled(self, pct: i32) -> Action {
        let m = |v: i32| ((v as i64 * pct as i64) / 100) as i32;
        match self {
            Action::Damage { amount, kind, target } => {
                Action::Damage { amount: m(amount), kind, target }
            }
            Action::MindDamage { amount, target } => {
                Action::MindDamage { amount: m(amount), target }
            }
            Action::GainMana(n) => Action::GainMana(m(n)),
            Action::Gain { what, amount } => Action::Gain { what, amount: m(amount) },
            Action::GainArmor(n) => Action::GainArmor(m(n)),
            Action::Grow(n) => Action::Grow(m(n)),
            Action::ReduceCooldown(ms) => {
                Action::ReduceCooldown(((ms as i64 * pct as i64) / 100) as u32)
            }
            // Stacks and curses are not quantities of anything - a stack is a
            // stack and a curse is a curse - so a multiplier has nothing to
            // multiply.
            other => other,
        }
    }
}

/// Fires every time the item this piece belongs to activates.
#[derive(Copy, Clone, Debug)]
pub enum Trigger {
    /// Unconditional.
    OnActivate(Action),
    /// Try to spend `cost` mana. Which branch runs is the whole point: the
    /// failure case is usually a penalty, so mana income becomes a real
    /// constraint rather than a nice-to-have.
    SpendMana { cost: i32, on_success: Action, on_failure: Action },
    /// Spend the run's own gold, mid-fight, for an effect that grows every
    /// time it pays.
    ///
    /// The only thing in the game that reaches out of the fight and into the
    /// purse: what this spends is gone when you get to the shop. `budget` is
    /// the most it will spend in one fight, so the worst case is knowable
    /// before you equip it, and both the budget and the escalation reset when
    /// the next fight starts. `on_success` is scaled by how many times it has
    /// paid - first payment at full, second at double, third at triple - which
    /// scales the outcome and never the cost, the same rule item power obeys.
    SpendGold { cost: i32, budget: i32, on_success: Action },
    /// Spend a banked pool. The mana version predates the others and is kept
    /// as its own variant so every existing component still reads the same.
    Spend { what: Resource, cost: i32, on_success: Action, on_failure: Action },
    /// Repeat `action` once per assembled item touching this one. With
    /// `same_slot_only`, only items in the same grid count.
    PerAdjacentItem { action: Action, same_slot_only: bool },
    /// Fires whenever an assembled item **touching this one** activates —
    /// reacting to a neighbour rather than to your own cooldown.
    OnAdjacentActivate(Action),
    /// Fires whenever an assembled item in a **different slot**, lying in the
    /// same rows as this one, activates. Rewards lining gear up across the
    /// five grids rather than only within one.
    OnAlignedActivate(Action),
    /// Fires once, before the first tick of the fight.
    ///
    /// Everything else in the game starts a fight at zero - armour, and all
    /// four pools - and has to earn its way up from there, which means the
    /// opening seconds of every fight look the same whatever you are wearing.
    /// This is the gear that shows up already holding something.
    OnBattleStart(Action),
    /// Spend the **whole** pool, and run `per` once for every `each` points
    /// it found. Nothing at all happens below `each`.
    ///
    /// Every other sink in the game is a fixed threshold: it takes the same
    /// amount whatever you have banked, so building a bigger reserve buys you
    /// nothing but more attempts. This is the one that reads the reserve. It
    /// makes holding a pool a decision rather than a waiting room, and it
    /// gives faith somewhere to go once the 40% resistance cap has stopped
    /// paying - which is otherwise the point where a faith build's income
    /// turns into dead weight.
    Consume { what: Resource, each: i32, per: Action },
    /// Run the wrapped trigger once per in-bounds empty cell touching this
    /// item.
    ///
    /// A build decision rather than a stat: room around an item is bought with
    /// the gear you did not pack there, and this is what pays for it. It wraps
    /// a trigger rather than an action so it composes with the spending ones -
    /// "for each open cell, spend 10 faith to gain a mana shield" is a repeat
    /// around a `Spend`.
    PerAdjacentEmpty(&'static Trigger),
    /// Fires when a **different spell in the same item** is cast.
    ///
    /// Only a crystal ball holds more than one spell, so this is what makes a
    /// ball worth more than the sum of its spells: the ones sitting idle this
    /// turn still answer the one that went off.
    OnOtherCast(Action),
    /// Fires whenever an assembled item **sharing only a corner** with this one
    /// activates.
    ///
    /// Adjacency is edge-sharing, so an item packed against three neighbours
    /// has already spent its sides. A diagonal is the relation left over: it
    /// sees past the things touching it, which is why it is the mind's and the
    /// hands' to use and nothing else's.
    OnDiagonalActivate(Action),
    /// Count something, and do something every `count` times it is seen.
    ///
    /// Every other trigger answers one event. This one answers a *number* of
    /// them, which is the only way the board gets to reward a long fight
    /// without also rewarding a fast one: ten activations is ten activations
    /// whether they took four seconds or forty.
    ///
    /// The counter belongs to the piece, resets when the fight does, and ticks
    /// after the event it watched has resolved. With `repeats` false it pays
    /// once and then stops counting.
    Watch { what: Watched, count: u32, then: Action, repeats: bool },
}

/// What a `Watch` counts.
///
/// Each is an event the fight already emits, so a watcher never needs the
/// engine to invent bookkeeping - it reads the same stream the log does.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Watched {
    /// Any other friendly item activating.
    AnyActivation,
    /// An edge-neighbour activating.
    AdjacentActivation,
    /// A corner-neighbour activating.
    DiagonalActivation,
    /// An item in another grid sharing this one's rows activating.
    AlignedActivation,
    /// A curse landing, on either side.
    CurseApplied,
}

impl Watched {
    pub fn name(self) -> &'static str {
        match self {
            Watched::AnyActivation => "activation",
            Watched::AdjacentActivation => "neighbour activation",
            Watched::DiagonalActivation => "diagonal activation",
            Watched::AlignedActivation => "aligned activation",
            Watched::CurseApplied => "curse",
        }
    }
}

impl Trigger {
    /// This trigger with its outcomes multiplied and its costs left alone.
    pub fn scaled(self, pct: i32) -> Trigger {
        match self {
            Trigger::OnActivate(a) => Trigger::OnActivate(a.scaled(pct)),
            Trigger::OnBattleStart(a) => Trigger::OnBattleStart(a.scaled(pct)),
            Trigger::OnAdjacentActivate(a) => Trigger::OnAdjacentActivate(a.scaled(pct)),
            Trigger::OnAlignedActivate(a) => Trigger::OnAlignedActivate(a.scaled(pct)),
            Trigger::OnOtherCast(a) => Trigger::OnOtherCast(a.scaled(pct)),
            Trigger::OnDiagonalActivate(a) => Trigger::OnDiagonalActivate(a.scaled(pct)),
            // The payload scales; the count is a count, and multiplying it
            // would make a powerful item wait longer for the same thing.
            Trigger::Watch { what, count, then, repeats } => {
                Trigger::Watch { what, count, then: then.scaled(pct), repeats }
            }
            Trigger::PerAdjacentItem { action, same_slot_only } => Trigger::PerAdjacentItem {
                action: action.scaled(pct),
                same_slot_only,
            },
            // The cost stays where it is; only what it buys grows.
            Trigger::SpendMana { cost, on_success, on_failure } => Trigger::SpendMana {
                cost,
                on_success: on_success.scaled(pct),
                on_failure: on_failure.scaled(pct),
            },
            // The cost and the budget are costs; only the payout scales.
            Trigger::SpendGold { cost, budget, on_success } => Trigger::SpendGold {
                cost,
                budget,
                on_success: on_success.scaled(pct),
            },
            Trigger::Spend { what, cost, on_success, on_failure } => Trigger::Spend {
                what,
                cost,
                on_success: on_success.scaled(pct),
                on_failure: on_failure.scaled(pct),
            },
            Trigger::Consume { what, each, per } => {
                Trigger::Consume { what, each, per: per.scaled(pct) }
            }
            // A repeat wraps a static trigger, so it cannot be rewritten in
            // place. It is expanded before dispatch and scaled there.
            Trigger::PerAdjacentEmpty(inner) => Trigger::PerAdjacentEmpty(inner),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Trigger::OnActivate(a) => format!("on activation, {}", a.describe()),
            Trigger::OnDiagonalActivate(a) => {
                format!("when an item touching only a corner of this one acts, {}", a.describe())
            }
            Trigger::Watch { what, count, then, repeats } => format!(
                "every {} {}{}, {}",
                count,
                what.name(),
                if *count == 1 { "" } else { "s" },
                then.describe(),
            ) + if *repeats { "" } else { " (once a fight)" },
            Trigger::SpendGold { cost, budget, on_success } => format!(
                "on activation, spend {} fnorp to {} - and again harder each time, \
                 up to {} fnorp a fight",
                cost,
                on_success.describe(),
                budget
            ),
            Trigger::SpendMana { cost, on_success, on_failure } => format!(
                "on activation, spend {} mana: if it works, {}; if not, {}",
                cost,
                on_success.describe(),
                on_failure.describe()
            ),
            Trigger::Spend { what, cost, on_success, on_failure } => format!(
                "on activation, spend {} {}: if it works, {}; if not, {}",
                cost,
                what.name(),
                on_success.describe(),
                on_failure.describe()
            ),
            Trigger::PerAdjacentItem { action, same_slot_only } => format!(
                "on activation, per adjacent assembled {}, {}",
                if *same_slot_only { "item in this slot" } else { "item" },
                action.describe()
            ),
            Trigger::OnBattleStart(a) => format!("at the start of the fight, {}", a.describe()),
            Trigger::Consume { what, each, per } => format!(
                "on activation, spend all your {}: per {} spent, {}",
                what.name(),
                each,
                per.describe()
            ),
            Trigger::PerAdjacentEmpty(inner) => {
                format!("per empty cell touching this item: {}", inner.describe())
            }
            Trigger::OnAdjacentActivate(a) => {
                format!("when a touching item activates, {}", a.describe())
            }
            Trigger::OnOtherCast(a) => {
                format!("when another spell in this item is cast, {}", a.describe())
            }
            Trigger::OnAlignedActivate(a) => format!(
                "when an item in another slot on the same rows activates, {}",
                a.describe()
            ),
        }
    }
}

/// A standing task carried by one component. It only counts while the piece
/// is part of an assembled item - a loose piece is inert, quests included -
/// and it is tallied from the combat log after a fight rather than during it,
/// so nothing in the simulation has to know quests exist.
#[derive(Copy, Clone, Debug)]
pub struct Quest {
    /// What the player is told to do.
    pub label: &'static str,
    /// How many times it has to happen.
    pub goal: u32,
    pub track: QuestTrack,
    /// The component this one turns into when the tally is met. Sometimes a
    /// straight upgrade of the same thing, sometimes a different piece of gear
    /// in a different slot entirely.
    pub becomes: &'static str,
}

/// What a quest counts.
#[derive(Copy, Clone, Debug)]
pub enum QuestTrack {
    /// Activations of the item this component is part of.
    SelfActivations,
    /// Activations of assembled items touching this one.
    AdjacentActivations,
    /// Activations of assembled items in another slot on the same rows, built
    /// from a component whose name contains `word`.
    AlignedActivations { word: &'static str },
    /// Curses this side landed on the enemy.
    CursesLanded,
}

impl QuestTrack {
    pub fn describe(self, goal: u32) -> String {
        match self {
            QuestTrack::SelfActivations => format!("go off {} times", goal),
            QuestTrack::AdjacentActivations => {
                format!("watch touching gear go off {} times", goal)
            }
            QuestTrack::AlignedActivations { word } => format!(
                "watch gear made with a \"{}\" go off {} times on its own rows",
                word, goal
            ),
            QuestTrack::CursesLanded => format!("land {} curses", goal),
        }
    }
}

/// Static definition of a component. Instances refer to these by index.
#[derive(Clone, Debug)]
pub struct PieceDef {
    pub name: &'static str,
    pub slot: SlotKind,
    pub kind: PieceKind,
    pub cells: &'static [(i8, i8)],
    /// Contributed whenever the piece is placed, assembled or not.
    pub base: Stats,
    /// Flat bonus, contributed only when this piece's item assembles.
    pub adjacency: Option<Adjacency>,
    /// Positional effect on (or from) neighbouring cells.
    pub effect: Option<Effect>,
    /// Base cooldown in milliseconds. Only meaningful on a core piece — the
    /// item it anchors inherits it. `0` means "use the slot's default".
    pub cooldown_ms: u32,
    /// A task the piece carries, and what it turns into on finishing it.
    pub quest: Option<Quest>,
    /// Hundredths of weapon power added to THIS item only, never to the
    /// wearer. What ink does: it scales the cast it is bound into.
    pub power_bonus: i32,
    /// Percentage points added to the item's speed. `+100` doubles the rate,
    /// halving the cooldown. Summed across the item's pieces.
    pub speed_bonus: i32,
    /// Fires each time this piece's item activates.
    pub triggers: &'static [Trigger],
    /// What the shop charges for it.
    pub price: i32,
}

impl PieceDef {
    /// Which grids this component may be placed in.
    ///
    /// Most gear belongs to one slot. Two kinds are shared, because the thing
    /// itself does not care where it goes: a material is leather or steel, and
    /// it will wrap a hand or a shin alike; plating is a shaped sheet, and it
    /// covers a head or a leg the same way.
    pub fn fits(&self, slot: SlotKind) -> bool {
        if self.slot == slot {
            return true;
        }
        match self.kind {
            PieceKind::Material => matches!(slot, SlotKind::Gloves | SlotKind::Greaves),
            PieceKind::Plating => matches!(slot, SlotKind::Helmet | SlotKind::Greaves),
            _ => false,
        }
    }

    /// Does this component go in more than one grid?
    ///
    /// Such a piece has no slot of its own until it is put somewhere, which is
    /// why it is drawn without a slot's colour until then.
    pub fn shared(&self) -> bool {
        SlotKind::ALL.iter().filter(|&&s| self.fits(s)).count() > 1
    }

    /// Every grid this component may go in, in slot order.
    pub fn slots(&self) -> Vec<SlotKind> {
        SlotKind::ALL.iter().copied().filter(|&s| self.fits(s)).collect()
    }
}

/// What a slot's recipes demand, as `(kind, min, max)` counts per item. A slot
/// can have more than one: an item counts as assembled if it satisfies any of
/// them.
///
/// The weapon slot has three. The martial one is the original. The other two
/// are spells - a book casts the one spell bound into it every time, while a
/// crystal ball holds several and casts a different one each time it comes
/// round, which is the whole difference between the two.
pub fn recipes(kind: SlotKind) -> &'static [&'static [(PieceKind, usize, usize)]] {
    match kind {
        SlotKind::Weapon => &[
            &[
                (PieceKind::Handle, 1, 1),
                (PieceKind::Damaging, 1, 2),
                (PieceKind::Accessory, 0, 2),
            ],
            &[
                (PieceKind::Book, 1, 1),
                (PieceKind::Ink, 1, 1),
                (PieceKind::Spell, 1, 1),
                (PieceKind::Accessory, 0, 1),
            ],
            // An orb takes no ink. Its spells are scaled by the alignment set
            // into it instead, which is a choice about *which* pool the whole
            // ball leans on rather than a flat multiplier.
            &[
                (PieceKind::Orb, 1, 1),
                (PieceKind::Spell, 2, 3),
                (PieceKind::Alignment, 0, 1),
            ],
        ],
        SlotKind::Helmet => &[&[
            (PieceKind::Frame, 1, 1),
            (PieceKind::Plating, 1, 2),
            (PieceKind::Crest, 0, 1),
        ]],
        SlotKind::Chest => &[&[(PieceKind::Base, 1, 1), (PieceKind::Layer, 1, 3)]],
        SlotKind::Gloves => &[&[
            (PieceKind::Material, 1, 1),
            (PieceKind::Mold, 1, 1),
            (PieceKind::Ring, 0, 2),
        ]],
        SlotKind::Greaves => &[&[
            (PieceKind::Material, 1, 1),
            (PieceKind::Mold, 1, 1),
            (PieceKind::Plating, 0, 1),
        ]]
    }
}

/// The first recipe for a slot, which is the one the rating treats as its
/// shape and the one the interface names.
pub fn recipe(kind: SlotKind) -> &'static [(PieceKind, usize, usize)] {
    recipes(kind)[0]
}

/// Cooldown used by a core piece that doesn't name its own, by slot. Weapons
/// swing quickly; armour ticks slowly.
pub fn default_cooldown_ms(slot: SlotKind) -> u32 {
    match slot {
        SlotKind::Weapon => 1500,
        SlotKind::Gloves => 3000,
        SlotKind::Greaves => 3500,
        SlotKind::Helmet => 4000,
        SlotKind::Chest => 5000,
    }
}

/// Handle to one physical component the player owns. Grids store these, never
/// the definition, so a multi-cell piece is the same id repeated across cells.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PieceId(pub u32);

impl std::fmt::Display for PieceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.0)
    }
}

#[derive(Clone, Debug)]
struct Instance {
    def: usize,
    /// Quarter turns clockwise applied to the definition's shape.
    rotation: u8,
}

/// Single source of truth for every component in play: which definition it
/// is, and how it is currently rotated.
#[derive(Clone, Default)]
pub struct PieceRegistry {
    instances: Vec<Instance>,
}

impl PieceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self, def: usize) -> PieceId {
        let id = PieceId(self.instances.len() as u32);
        self.instances.push(Instance { def, rotation: 0 });
        id
    }

    fn instance(&self, id: PieceId) -> &Instance {
        self.instances
            .get(id.0 as usize)
            .expect("missing piece instance")
    }

    pub fn def(&self, id: PieceId) -> &'static PieceDef {
        &CATALOG[self.instance(id).def]
    }

    /// Which catalog entry this instance is. Used by the name generator, so
    /// two copies of the same component hash identically.
    pub fn def_index(&self, id: PieceId) -> usize {
        self.instance(id).def
    }

    pub fn rotation(&self, id: PieceId) -> u8 {
        self.instance(id).rotation
    }

    /// The piece's footprint at its current rotation.
    pub fn shape(&self, id: PieceId) -> Shape {
        let inst = self.instance(id);
        Shape::new(CATALOG[inst.def].cells).rotated(inst.rotation)
    }

    pub fn rotate_cw(&mut self, id: PieceId) {
        let inst = &mut self.instances[id.0 as usize];
        inst.rotation = (inst.rotation + 1) % 4;
    }

    /// Turn one instance into a different component. Its identity, shape and
    /// everything else come from the new definition; the rotation is reset,
    /// since the old one meant nothing to the new shape.
    pub fn transform(&mut self, id: PieceId, def: usize) {
        let inst = &mut self.instances[id.0 as usize];
        inst.def = def;
        inst.rotation = 0;
    }

    pub fn set_rotation(&mut self, id: PieceId, rotation: u8) {
        self.instances[id.0 as usize].rotation = rotation % 4;
    }

    pub fn count(&self) -> usize {
        self.instances.len()
    }
}

// ---------------------------------------------------------------- content
//
// Plain Rust data. Every slot below is buildable from the starting inventory,
// and exactly one piece per slot carries an adjacency bonus.

pub static CATALOG: &[PieceDef] = &[
    // ---- Gear that is going somewhere ----
    //
    // Each of these carries a quest. It only ticks while the piece is part of
    // an assembled item, and finishing it turns the piece into the component
    // named in `becomes` - sometimes a straight upgrade, sometimes something
    // that does not even belong in the same slot any more.
    PieceDef {
        name: "Helm of Blades",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1)],
        base: Stats::health(90),
        adjacency: None,
        effect: None,
        cooldown_ms: 3800,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainArmor(6))],
        quest: Some(Quest {
            label: "Helm of Blades",
            goal: 10,
            track: QuestTrack::AlignedActivations { word: "Blade" },
            becomes: "Blade of Helms",
        }),
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Blade of Helms",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        // It kept the helm's job and lost the blade's: it sits in a weapon and
        // gives armour where a damaging piece would give damage.
        base: Stats::ZERO,
        adjacency: Some(Adjacency {
            label: "Blade of Helms: +175 health",
            stats: Stats::health(175),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainArmor(22))],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    PieceDef {
        name: "Apprentice's Primer",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 2,
            on_success: Action::Damage {
                amount: 9,
                kind: DamageType::Magic,
                target: Target::Enemy,
            },
            on_failure: Action::GainMana(2),
        }],
        quest: Some(Quest {
            label: "Apprentice's Primer",
            goal: 20,
            track: QuestTrack::SelfActivations,
            becomes: "Archmage's Primer",
        }),
        power_bonus: 40,
        price: 9,
    },
    PieceDef {
        name: "Archmage's Primer",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::mana(2),
        adjacency: None,
        effect: None,
        cooldown_ms: 1700,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 5,
            on_success: Action::GainEmpowerment(1),
            on_failure: Action::GainMana(3),
        }],
        quest: None,
        power_bonus: 160,
        price: 34,
    },
    PieceDef {
        name: "Cracked Pauldron",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (1, 1)],
        base: Stats::health(50),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: Some(Quest {
            label: "Cracked Pauldron",
            goal: 25,
            track: QuestTrack::AdjacentActivations,
            becomes: "Warlord's Pauldron",
        }),
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Warlord's Pauldron",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (1, 1)],
        base: Stats::health(240),
        adjacency: Some(Adjacency {
            label: "Warlord: +6 strength",
            stats: Stats::strength(6),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 32,
    },
    PieceDef {
        name: "Hexer's Tally",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Frost,
            target: Target::Enemy,
        })],
        quest: Some(Quest {
            label: "Hexer's Tally",
            goal: 12,
            track: QuestTrack::CursesLanded,
            becomes: "Hexer's Reckoning",
        }),
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Hexer's Reckoning",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats::ZERO,
        adjacency: Some(Adjacency {
            label: "Reckoning: 30% curse resistance",
            stats: Stats { curse_resist: 30, ..Stats::ZERO },
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[
            Trigger::OnActivate(Action::Curse {
                kind: CurseKind::Searing,
                target: Target::Enemy,
            }),
            Trigger::OnActivate(Action::Curse {
                kind: CurseKind::Frost,
                target: Target::Enemy,
            }),
        ],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    PieceDef {
        name: "Wayfarer's Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats::regen(1),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 20,
        triggers: &[],
        quest: Some(Quest {
            label: "Wayfarer's Sole",
            goal: 15,
            track: QuestTrack::SelfActivations,
            becomes: "Sevenleague Sole",
        }),
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Sevenleague Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats::regen(4),
        adjacency: Some(Adjacency {
            label: "Sevenleague: +0.35x weapon power",
            stats: Stats::power(35),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 60,
        triggers: &[Trigger::OnActivate(Action::GainMana(2))],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    // ---- Typed damage, banked resources, and more ways into a spell ----
    //
    // Deliberately spread across every slot and every axis the class system
    // measures: a build cannot lean hard enough on magic, iron, rage, faith or
    // growth to earn a class unless there is gear to lean on.
    PieceDef {
        name: "Emberplate",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { magic_damage: 7, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Runic Weave",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { magic_resist: 18, magic_harden: 15, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Voidsilk Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(0,1),(2,1),(0,2),(1,2),(2,2)],
        base: Stats { health: 150, magic_damage: 6, mana: 2, ..Stats::ZERO },
        adjacency: Some(Adjacency { label: "Voidsilk: +20% magic resist", stats: Stats { magic_resist: 20, ..Stats::ZERO } }),
        effect: None,
        cooldown_ms: 3600,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    PieceDef {
        name: "Starlit Mantle",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { magic_damage: 9, magic_pierce: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Leyline Cuirass",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { health: 200, magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Mana, amount: 3 })],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    PieceDef {
        name: "Spiked Vambrace",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { physical_damage: 8, physical_pierce: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Ironhide Wrap",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { physical_resist: 20, physical_harden: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3400,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Breaker's Fist",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1),(0,2),(1,2)],
        base: Stats { physical_damage: 14, physical_pierce: 35, ..Stats::ZERO },
        adjacency: Some(Adjacency { label: "Breaker: +6 strength", stats: Stats { strength: 6, ..Stats::ZERO } }),
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 26,
    },
    PieceDef {
        name: "Tempered Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { physical_resist: 16, physical_harden: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Warplate Greave",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 125, physical_resist: 22, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainArmor(14))],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    PieceDef {
        name: "Bloodrage Grip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2),(0,3)],
        base: Stats { physical_damage: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 1400,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Rage, amount: 2 })],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Fury Sigil",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0)],
        base: Stats { rage: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Berserker's Plate",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Wrathful Talons",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Rage,
            cost: 4,
            on_success: Action::Damage { amount: 22, kind: DamageType::Physical, target: Target::Enemy },
            on_failure: Action::Gain { what: Resource::Rage, amount: 2 },
        }],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Cull",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { physical_damage: 16, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 10,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Rage, amount: 1 })],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    PieceDef {
        name: "Votive Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(0,1)],
        base: Stats { faith: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Reliquary Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(1,1),(1,2)],
        base: Stats { health: 150, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3600,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Faith, amount: 2 })],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    PieceDef {
        name: "Consecrated Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { magic_resist: 15, physical_resist: 15, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Absolution",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { magic_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Faith,
            cost: 3,
            on_success: Action::GainArmor(30),
            on_failure: Action::Gain { what: Resource::Faith, amount: 1 },
        }],
        quest: None,
        power_bonus: 0,
        price: 21,
    },
    PieceDef {
        name: "Pilgrim's Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1)],
        base: Stats { faith: 1, regen: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Rootbound Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Nature, amount: 2 })],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Verdant Weave",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { nature: 1, regen: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Bloomcap",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats { nature: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Wildgrowth",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { magic_damage: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Nature,
            cost: 3,
            on_success: Action::GainMana(6),
            on_failure: Action::Gain { what: Resource::Nature, amount: 2 },
        }],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Thornweald Grip",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { nature: 2, physical_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Astrolabe",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(1,0),(0,1),(1,1),(2,1),(1,2)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2200,
        speed_bonus: 0,
        // It reads the clock, and sometimes stops theirs.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::ReduceCooldown(250),
        }],
        quest: None,
        power_bonus: 45,
        price: 16,
    },
    PieceDef {
        name: "Obsidian Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { magic_damage: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Damage {
            amount: 7,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 70,
        price: 15,
    },
    PieceDef {
        name: "Prismatic Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Mana, amount: 1 })],
        quest: None,
        power_bonus: 130,
        price: 14,
    },
    PieceDef {
        name: "Shatterbolt",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { magic_damage: 13, magic_pierce: 40, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Curse {
            kind: CurseKind::Stun,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Hoarfrost",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats { magic_damage: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Frost, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Timeworn Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { magic_damage: 2, mana: 2, ..Stats::ZERO },
        adjacency: Some(Adjacency { label: "Timeworn: +0.30x weapon power", stats: Stats { power: 30, ..Stats::ZERO } }),
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        // Worn thin enough that time leaks through it.
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::ReduceCooldown(300),
        }],
        quest: None,
        power_bonus: 65,
        price: 21,
    },
    // ---- Gear that reads its neighbours ----
    //
    // `DoubleNeighbor` reaches out and changes what the pieces around it are
    // worth. These do the opposite: they read what is packed against them and
    // change what THEY are worth, so a component can reward a particular sort
    // of company.
    PieceDef {
        name: "Multi-Handle",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(1,0),(0,1),(1,1),(0,2),(1,2)],
        base: Stats::ZERO,
        adjacency: None,
        effect: Some(Effect { label: "Multi-Handle: +2 strength per adjacent damaging piece", when: When::Assembled, kind: EffectKind::SelfPerNeighborKind { kind: PieceKind::Damaging, stat: StatKind::Strength, per: 2 } }),
        cooldown_ms: 1800,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Reliquary Frame of Nine",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { health: 100, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Nine: +70 health per adjacent plating", when: When::Assembled, kind: EffectKind::SelfPerNeighborKind { kind: PieceKind::Plating, stat: StatKind::Health, per: 70 } }),
        cooldown_ms: 3400,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 21,
    },
    PieceDef {
        name: "Layered Core",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 125, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Layered: +60 health per adjacent layer", when: When::Assembled, kind: EffectKind::SelfPerNeighborKind { kind: PieceKind::Layer, stat: StatKind::Health, per: 60 } }),
        cooldown_ms: 3600,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    PieceDef {
        name: "Knuckleduster",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: Some(Effect { label: "Knuckleduster: +45 health per adjacent ring", when: When::Assembled, kind: EffectKind::SelfPerNeighborKind { kind: PieceKind::Ring, stat: StatKind::Health, per: 45 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Grimoire Rack",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: Some(Effect { label: "Rack: +0.15x power per adjacent spell", when: When::Assembled, kind: EffectKind::SelfPerNeighborKind { kind: PieceKind::Spell, stat: StatKind::Power, per: 15 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Studded Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats::ZERO,
        adjacency: None,
        effect: Some(Effect { label: "Studded: +40 health per adjacent material", when: When::Assembled, kind: EffectKind::SelfPerNeighborKind { kind: PieceKind::Material, stat: StatKind::Health, per: 40 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    // ---- Rings ----
    //
    // A glove takes up to two. They are one cell each and cost little, which
    // makes them the thing you slot into whatever corner is left over.
    PieceDef {
        name: "Signet of Vigour",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { health: 110, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Iron Band",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { strength: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Ring of Tides",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // The tide goes out from under them.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::GainMana(1),
        }],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Emberloop",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Bloodring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { rage: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Warding Ring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { physical_resist: 10, magic_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Ring of Hours",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0),(1,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 35,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Seal of the Grove",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { nature: 1, regen: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Oathring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { faith: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Piercer's Band",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { physical_pierce: 20, magic_pierce: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    // ---- Pace, and the answer to it ----
    //
    // Trigger speed used to live almost entirely on weapons, so how fast a
    // build ran was a weapon-slot decision alone. These spread it across the
    // other four - and, since a build that outruns you had no answer, so do
    // the things that slow gear down and blunt a heavy hit.
    PieceDef {
        name: "Swiftplate",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 45,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Lightweave",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 40,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Deft Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 55,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Quickstep Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 50,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Hastening Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: Some(Adjacency { label: "Hastening: +0.20x weapon power", stats: Stats { power: 20, ..Stats::ZERO } }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 30,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Rimeguard Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { health: 200, magic_resist: 15, ..Stats::ZERO },
        adjacency: Some(Adjacency { label: "Rimeguard: 20% magic hardening", stats: Stats { magic_harden: 20, ..Stats::ZERO } }),
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Frost, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 25,
    },
    PieceDef {
        name: "Tarpit Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Frost, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Stonewall Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { health: 275, physical_resist: 18, physical_harden: 25, ..Stats::ZERO },
        adjacency: Some(Adjacency { label: "Stonewall: +25% physical resistance", stats: Stats { physical_resist: 25, ..Stats::ZERO } }),
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainArmor(26))],
        quest: None,
        power_bonus: 0,
        price: 28,
    },
    PieceDef {
        name: "Anchor Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { physical_harden: 30, magic_harden: 30, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainShield(1))],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    PieceDef {
        name: "Bulwark Vial",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(0,1)],
        base: Stats { armor: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    // ---- Gear for the thin axes ----
    //
    // The class fingerprint measures curses, growth, faith, rage and spell
    // cores, and the catalogue supplied so little of each that five classes
    // could not be reached by any build. These exist to give those axes
    // something to read.
    PieceDef {
        name: "Hexbrand",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { magic_damage: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Searing, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Coven Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Frost, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Blight Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Searing, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Malefic Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Frost, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Plaguewalkers",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { magic_damage: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Searing, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    PieceDef {
        name: "Heartwood Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { health: 175, nature: 2, ..Stats::ZERO },
        adjacency: Some(Adjacency { label: "Heartwood: +4 regen a second", stats: Stats { regen: 4, ..Stats::ZERO } }),
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    PieceDef {
        name: "Sapling Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1)],
        base: Stats { nature: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Bloomguard",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { nature: 2, regen: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Nature, amount: 2 })],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Green Crown",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { nature: 1, health: 100, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Nature, amount: 2 })],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    PieceDef {
        name: "Oathplate",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { faith: 2, physical_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Chapel Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(0,1),(2,1)],
        base: Stats { faith: 1, health: 125, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Faith, amount: 2 })],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Zealot's Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { faith: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Bulwark Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(3,0),(0,1),(1,1),(2,1),(3,1)],
        base: Stats { health: 225, physical_resist: 22, physical_damage: 6, ..Stats::ZERO },
        adjacency: Some(Adjacency { label: "Bulwark: +20% physical hardening", stats: Stats { physical_harden: 20, ..Stats::ZERO } }),
        effect: None,
        cooldown_ms: 3400,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 26,
    },
    PieceDef {
        name: "Riveted Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { physical_resist: 16, physical_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Warcry Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(0,1)],
        base: Stats { rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Ravener's Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Rage, amount: 2 })],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Runebound Tome",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(1,0),(0,1),(1,1),(0,2),(1,2)],
        base: Stats { magic_damage: 4, mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2400,
        speed_bonus: 0,
        // The rune holds them still for as long as it holds.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 110,
        price: 22,
    },
    PieceDef {
        name: "Seer's Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(1,0),(0,1),(1,1),(2,1)],
        base: Stats { magic_damage: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2400,
        speed_bonus: 0,
        // It has seen what they were going to do.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Misfire, target: Target::Enemy },
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 70,
        price: 17,
    },
    PieceDef {
        name: "Starfall",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { magic_damage: 16, magic_pierce: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Damage {
            amount: 9,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 23,
    },
    // ---- The deep end ----
    //
    // Gear meant for the harder difficulties. Prices come from the rating, so
    // these are expensive by construction rather than by hand.
    PieceDef {
        name: "Godsteel Haft",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)],
        base: Stats::power(70),
        adjacency: Some(Adjacency {
            label: "Godsteel: +8 strength",
            stats: Stats::strength(8),
        }),
        effect: None,
        cooldown_ms: 3600,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 40,
    },
    PieceDef {
        name: "Sunderer",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)],
        base: Stats::physical(34),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 15,
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Searing,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 44,
    },
    PieceDef {
        name: "Aegis Crown",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (2, 2)],
        base: Stats::health(300),
        adjacency: Some(Adjacency {
            label: "Aegis: 25% mind and curse resistance",
            stats: Stats { mind_resist: 25, curse_resist: 25, ..Stats::ZERO },
        }),
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainArmor(16))],
        quest: None,
        power_bonus: 0,
        price: 38,
    },
    PieceDef {
        name: "Adamant Carapace",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1), (3, 1), (0, 2), (3, 2)],
        base: Stats::health(450),
        adjacency: Some(Adjacency {
            label: "Adamant: +3 regen a second",
            stats: Stats::regen(3),
        }),
        effect: None,
        cooldown_ms: 3400,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainArmor(30))],
        quest: None,
        power_bonus: 0,
        price: 46,
    },
    PieceDef {
        name: "Titan's Grip",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats::strength(14),
        adjacency: Some(Adjacency {
            label: "Titan: +0.60x weapon power",
            stats: Stats::power(60),
        }),
        effect: None,
        cooldown_ms: 2400,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainMana(3))],
        quest: None,
        power_bonus: 0,
        price: 42,
    },
    PieceDef {
        name: "Sevenleague Boots",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)],
        base: Stats::regen(5),
        adjacency: Some(Adjacency {
            label: "Sevenleague: +225 health",
            stats: Stats::health(225),
        }),
        effect: None,
        cooldown_ms: 2200,
        speed_bonus: 25,
        triggers: &[Trigger::OnActivate(Action::GainArmor(12))],
        quest: None,
        power_bonus: 0,
        price: 40,
    },
    // ---- Weapon, the arcane way: books, orbs, inks and spells ----
    //
    // A spell is built the same way a weapon is: one core that sets the
    // cadence, something that scales the payload, and the payload itself. The
    // difference is only which recipe it answers to.
    PieceDef {
        name: "Pocket Grimoire",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 1600,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainMana(1))],
        quest: None,
        power_bonus: 0,
        price: 5,
    },
    PieceDef {
        name: "Leaden Tome",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        base: Stats::health(125),
        // Slow, heavy, and worth it: the ink bound into it lands harder.
        adjacency: Some(Adjacency {
            label: "Leaden: +1.20x to this cast",
            stats: Stats::ZERO,
        }),
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        // Heavy going, for whoever it lands on.
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Frost, target: Target::Enemy })],
        quest: None,
        power_bonus: 120,
        price: 14,
    },
    PieceDef {
        name: "Chained Codex",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        // It used to read off its neighbours, which is the hands' work now. A
        // plain book, and priced like one.
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Scrying Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(1, 0), (2, 0), (0, 1), (1, 1), (2, 1), (3, 1), (1, 2), (2, 2)],
        base: Stats::mana(1),
        adjacency: None,
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::GainMana(1))],
        quest: None,
        power_bonus: 35,
        price: 13,
    },
    PieceDef {
        name: "Hollow Sphere",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(1, 0), (0, 1), (2, 1), (1, 2)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 1900,
        speed_bonus: 0,
        // The hole in the middle is the point: it wants room.
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::OnActivate(Action::Damage {
            amount: 6,
            kind: DamageType::Magic,
            target: Target::Enemy,
        }))],
        quest: None,
        power_bonus: 55,
        price: 9,
    },
    PieceDef {
        name: "Soot Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0, 0), (0, 1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Searing, target: Target::Enemy })],
        quest: None,
        power_bonus: 90,
        price: 5,
    },
    PieceDef {
        name: "Quicksilver Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0, 0), (1, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Potent, but it wants paying for.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::GainEmpowerment(1),
            on_failure: Action::Curse { kind: CurseKind::Frost, target: Target::Yourself },
        }],
        quest: None,
        power_bonus: 170,
        price: 12,
    },
    PieceDef {
        name: "Bloodletter's Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Paid in your own maximum health, which never comes back.
        triggers: &[Trigger::OnActivate(Action::MindDamage {
            amount: 2,
            target: Target::Yourself,
        })],
        quest: None,
        power_bonus: 240,
        price: 16,
    },
    PieceDef {
        name: "Emberburst",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(1, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats::magic(14),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Searing,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Rime Nova",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::magic(7),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Frost,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Siphon",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats::mind(4),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Eats their maximum health and hands you the mana back.
        triggers: &[Trigger::OnActivate(Action::GainMana(3))],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Warding Sigil",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0, 0), (1, 0), (1, 1)],
        base: Stats::armor(9),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // It shields whatever else the ball is doing.
        triggers: &[Trigger::OnOtherCast(Action::GainArmor(7))],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Arc Lightning",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0, 0), (1, 0), (1, 1), (2, 1)],
        base: Stats::magic(9),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Arcs to the next voice in the ball. It used to count the finished
        // gear around it, which is what a glove does now.
        triggers: &[Trigger::OnOtherCast(Action::Damage {
            amount: 6,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Mirrorcast",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0, 0), (1, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Does nothing on its own; it answers whatever else the ball cast.
        // It used to answer the neighbouring item instead - answering across
        // the board is the gloves' tense now, and answering inside your own
        // item is the weapon's.
        triggers: &[Trigger::OnOtherCast(Action::Damage {
            amount: 7,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    // ---- Weapon: handles, damaging pieces, accessories ----
    PieceDef {
        name: "Oak Handle",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats::power(20),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Balanced Grip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0, 0), (0, 1), (0, 2), (0, 3)],
        base: Stats::power(10),
        // >>> the Weapon slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Balanced: +0.50x weapon power",
            stats: Stats::power(50),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Iron Blade",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (0, 2), (0, 3)],
        base: Stats { physical_damage: 8, ..Stats::new(0, 2, 0, 80) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Serrated Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(1, 0), (1, 1), (0, 1), (1, 2)],
        base: Stats { physical_damage: 6, ..Stats::new(0, 4, 0, 60) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Ruby Inlay",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0)],
        base: Stats::strength(3),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 5,
    },
    PieceDef {
        name: "Balance Weight",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0), (1, 0)],
        base: Stats::power(25),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    // ---- Helmet: frame, plating, crest ----
    PieceDef {
        name: "Steel Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1)],
        base: Stats { armor: 12, ..Stats::health(50) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 5,
    },
    PieceDef {
        name: "Iron Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 9, ..Stats::health(75) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 5,
    },
    PieceDef {
        name: "Visor of Focus",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0)],
        base: Stats::health(25),
        // >>> the Helmet slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Focused: +3 strength",
            stats: Stats::strength(3),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Crest of Vigor",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0), (0, 1)],
        base: Stats { mana: 2, ..Stats::regen(1) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    // ---- Chest: one base, up to three layers ----
    PieceDef {
        name: "Padded Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[
            (0, 0), (1, 0), (2, 0), (3, 0),
            (0, 1), (1, 1), (2, 1), (3, 1),
            (0, 2), (1, 2), (2, 2), (3, 2),
        ],
        base: Stats { armor: 16, ..Stats::health(125) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Chain Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats { armor: 7, ..Stats::health(60) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    PieceDef {
        name: "Plate Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats { armor: 10, ..Stats::health(90) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 5,
    },
    PieceDef {
        name: "Woven Underlayer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats::health(30),
        // >>> the Chest slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Woven: +2 regen",
            stats: Stats::regen(2),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    // ---- Gloves: material + mold ----
    PieceDef {
        name: "Leather Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { armor: 6, ..Stats::strength(2) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Steel Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)],
        base: Stats { armor: 9, ..Stats::new(5, 4, 0, 0) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Gauntlet Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (0, 1), (0, 2), (1, 2)],
        base: Stats::strength(1),
        // >>> the Gloves slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Gauntleted: +2 strength",
            stats: Stats::strength(2),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Gripping Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats { mana: 2, ..Stats::power(15) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    // ---- Greaves: material + mold ----
    PieceDef {
        name: "Runed Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { armor: 9, ..Stats::health(25) },
        // >>> the Greaves slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Runed: +75 health",
            stats: Stats::health(75),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Boiled Leather",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 12, ..Stats::health(50) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    PieceDef {
        name: "Greave Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (1, 1), (1, 2)],
        base: Stats::regen(1),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Runner's Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { mana: 2, ..Stats::regen(2) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    // ---- Components with positional effects ----
    PieceDef {
        name: "Runed Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        // A cross-ish blade, so it can touch accessories on several sides.
        cells: &[(0, 0), (0, 1), (0, 2), (1, 1)],
        base: Stats { physical_damage: 5, ..Stats::new(0, 1, 0, 45) },
        adjacency: None,
        effect: Some(Effect {
            label: "adjacent accessories give double strength",
            when: When::Assembled,
            kind: EffectKind::DoubleNeighbor {
                kind: PieceKind::Accessory,
                stat: StatKind::Strength,
            },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Hollow Weave",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats::health(20),
        adjacency: None,
        effect: Some(Effect {
            label: "+1 strength per empty cell touching it",
            when: When::Always,
            kind: EffectKind::SelfPerEmptyCell { stat: StatKind::Strength, per: 1 },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Unbound Core",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::health(40),
        adjacency: None,
        effect: Some(Effect {
            label: "adjacent layers give double health",
            when: When::NotAssembled,
            kind: EffectKind::DoubleNeighbor {
                kind: PieceKind::Layer,
                stat: StatKind::Health,
            },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    // ---- Cursed line: powerful, but they bite back ----
    PieceDef {
        name: "Cursed Handle",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats::power(30),
        adjacency: None,
        effect: Some(Effect {
            label: "other assembled items touching it give double strength",
            when: When::Assembled,
            kind: EffectKind::DoubleAdjacentItemStat { stat: StatKind::Strength },
        }),
        // 0.5 attacks a second.
        cooldown_ms: 2000,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 5,
            on_success: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
            on_failure: Action::Curse { kind: CurseKind::Frost, target: Target::Yourself },
        }],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Cursed Blade",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (1, 1), (0, 2)],
        base: Stats::physical(10),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        // Doubles the rate of whatever weapon it is built into.
        speed_bonus: 100,
        // It burns whoever swings it, once a swing. It used to burn you once
        // per item packed beside it, which was the hands' way of counting.
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Searing,
            target: Target::Yourself,
        })],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    // ---- Spares, so every slot can host more than one finished item ----
    PieceDef {
        name: "Bone Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1)],
        base: Stats { armor: 10, mana: 1, ..Stats::new(6, 0, 1, 0) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    PieceDef {
        name: "Hide Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 12, ..Stats::health(70) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 5,
    },
    // ================= MAGE LINE: makes and spends mana =================
    PieceDef {
        name: "Mage's Rod",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0, 0), (0, 1), (0, 2), (0, 3)],
        base: Stats { mana: 3, ..Stats::power(10) },
        adjacency: None,
        effect: None,
        cooldown_ms: 2500,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Arcane Splinter",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (1, 1)],
        base: Stats { magic_damage: 3, ..Stats::new(0, 0, 0, 20) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Cheap to fire, brutal when the mana is there.
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Damage { amount: 18, kind: DamageType::Magic, target: Target::Enemy },
            on_failure: Action::GainMana(1),
        }],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Mana Loom",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2)],
        base: Stats { mana: 6, armor: 10, ..Stats::health(90) },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Mage's Circlet",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1)],
        base: Stats { mana: 4, ..Stats::health(40) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Runed Lining",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats { mana: 3, ..Stats::health(30) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Misfire, target: Target::Enemy },
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Mage's Wrapping",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { mana: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2500,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Mage's Sandals",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats { mana: 3, ..Stats::health(20) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Scrying Lens",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0)],
        base: Stats { mind: 3, mind_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Overflow Vial",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: Some(Adjacency {
            label: "Overflowing: +2 mana",
            stats: Stats { mana: 2, ..Stats::ZERO },
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },

    // ================ WITCH LINE: pays in curses ================
    PieceDef {
        name: "Witch's Crook",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0, 0), (0, 1), (0, 2), (1, 0)],
        base: Stats { curse_resist: 10, ..Stats::power(20) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
            on_failure: Action::Curse { kind: CurseKind::Frost, target: Target::Yourself },
        }],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Hexbolt",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats { magic_damage: 7, mind: 2, ..Stats::new(0, 0, 0, 40) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Witch's Hat",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(1, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        base: Stats { curse_resist: 15, ..Stats::health(50) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3500,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Frost,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Hexweave Shroud",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        base: Stats { curse_resist: 20, armor: 10, ..Stats::health(80) },
        adjacency: None,
        effect: None,
        cooldown_ms: 4500,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
            on_failure: Action::GainArmor(4),
        }],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Witch's Claw",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (0, 2)],
        base: Stats { curse_resist: 5, ..Stats::strength(2) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Frost,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Hexer's Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (1, 1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
            on_failure: Action::GainMana(1),
        }],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Witch's Stilts",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (0, 1), (0, 2), (1, 2)],
        base: Stats { curse_resist: 15, ..Stats::health(40) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3500,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Bileglass Vial",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0), (1, 0)],
        base: Stats { mind: 1, ..Stats::ZERO },
        adjacency: Some(Adjacency {
            label: "Bilious: +2 mind damage",
            stats: Stats::mind(2),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Coven Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0), (0, 1)],
        base: Stats { curse_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAlignedActivate(Action::Curse {
            kind: CurseKind::Searing,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 11,
    },

    // ============ REACTIVE: gear that answers other gear ============
    PieceDef {
        name: "Quickening Charm",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Quickens the ball it is set into, not the gear beside it.
        triggers: &[Trigger::OnOtherCast(Action::ReduceCooldown(1000))],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Chain Coil",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0), (0, 1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Follows the cast rather than the neighbour: a chain that whips out
        // after whatever the ball just threw.
        triggers: &[Trigger::OnOtherCast(Action::Damage {
            amount: 5,
            kind: DamageType::Physical,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Channeling Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Line these gloves up with gear in another slot and every time that
        // gear fires, you bank a point of mana.
        triggers: &[Trigger::OnAlignedActivate(Action::GainMana(1))],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Striding Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (1, 1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAlignedActivate(Action::ReduceCooldown(500))],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Thornmail Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0)],
        base: Stats { armor: 9, ..Stats::health(40) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAdjacentActivate(Action::Damage {
            amount: 3,
            kind: DamageType::Physical,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Third Eye",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0)],
        base: Stats { mind: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAdjacentActivate(Action::GainMana(1))],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Ember Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0), (1, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAlignedActivate(Action::Damage {
            amount: 2,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Grave-Iron Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (2, 0), (2, 1)],
        base: Stats { armor: 11, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Featherweight Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 60,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Warding Plate",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { armor: 14, curse_resist: 10, ..Stats::health(40) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Mirrored Visor",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0), (1, 1)],
        base: Stats { mind_resist: 25, ..Stats::health(30) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Ironbark Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { armor: 16, ..Stats::health(50) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Duelist's Grip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0, 0), (0, 1)],
        base: Stats::power(15),
        adjacency: None,
        effect: None,
        cooldown_ms: 900,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Executioner's Haft",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)],
        base: Stats::power(90),
        adjacency: None,
        effect: None,
        cooldown_ms: 4500,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Bonesaw",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (1, 0), (1, 1), (2, 1)],
        base: Stats { physical_damage: 9, ..Stats::new(0, 3, 0, 30) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 20,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Whetstone",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0)],
        base: Stats::strength(4),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Pathfinder Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (2, 0)],
        base: Stats { armor: 7, ..Stats::regen(2) },
        adjacency: None,
        effect: None,
        cooldown_ms: 2500,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Bulwark Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 14, ..Stats::strength(3) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3500,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },

    // ====== OVERSIZED: hopeless to build, formidable left in bits ======
    PieceDef {
        name: "Vast Tapestry",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        // 5x4 solid: fills most of a chest grid, leaving nowhere for a base.
        cells: &[
            (0, 0), (1, 0), (2, 0), (3, 0), (4, 0),
            (0, 1), (1, 1), (2, 1), (3, 1), (4, 1),
            (0, 2), (1, 2), (2, 2), (3, 2), (4, 2),
            (0, 3), (1, 3), (2, 3), (3, 3), (4, 3),
        ],
        base: Stats::health(30),
        adjacency: None,
        effect: Some(Effect {
            label: "Unbound: +550 health while it stays loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat { stats: Stats::health(550) },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Colossus Ring",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        // A hollow 5x5 ring. Nothing fits through the middle either.
        cells: &[
            (0, 0), (1, 0), (2, 0), (3, 0), (4, 0),
            (0, 1), (4, 1),
            (0, 2), (4, 2),
            (0, 3), (4, 3),
            (0, 4), (1, 4), (2, 4), (3, 4), (4, 4),
        ],
        base: Stats::health(40),
        adjacency: None,
        effect: Some(Effect {
            label: "Unbound: +300 health and +9 strength while it stays loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat {
                stats: Stats { ..Stats::new(60, 9, 0, 0) },
            },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Sprawling Handwrap",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        // A five-armed spider. Almost impossible to leave room for a mold.
        cells: &[
            (2, 0),
            (0, 1), (1, 1), (2, 1), (3, 1), (4, 1),
            (2, 2),
            (1, 3), (3, 3),
            (0, 4), (4, 4),
        ],
        base: Stats::strength(2),
        adjacency: None,
        effect: Some(Effect {
            label: "Unbound: +14 strength while it stays loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat { stats: Stats::strength(14) },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Wandering Root",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        // A staircase across the whole grid.
        cells: &[
            (0, 0), (0, 1), (1, 1), (1, 2), (2, 2), (2, 3),
            (3, 3), (3, 4), (4, 4), (4, 5), (5, 5),
        ],
        base: Stats::regen(1),
        adjacency: None,
        effect: Some(Effect {
            label: "Unbound: +6 regen and +150 health while it stays loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat { stats: Stats { ..Stats::new(30, 0, 6, 0) } },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Broken Crown",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        // Jagged and wide; a frame rarely fits beside it.
        cells: &[
            (0, 0), (2, 0), (4, 0),
            (0, 1), (1, 1), (2, 1), (3, 1), (4, 1),
            (0, 2), (4, 2),
        ],
        base: Stats::health(25),
        adjacency: None,
        effect: Some(Effect {
            label: "Unbound: +200 health and +20% both resistances while loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat {
                stats: Stats { mind_resist: 20, curse_resist: 20, ..Stats::health(200) },
            },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    // ===== MANA BUFFS: pay mana for a stack that scales off the mana left =====
    PieceDef {
        name: "Empowering Focus",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::GainEmpowerment(1),
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Empowering Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (0, 1), (1, 1)],
        base: Stats { mana: 1, armor: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::GainEmpowerment(1),
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Mana Ward",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0), (1, 1)],
        base: Stats { armor: 8, mana: 1, ..Stats::health(30) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::GainShield(1),
            on_failure: Action::GainArmor(8),
        }],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Aegis Weave",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 12, mana: 2, ..Stats::health(50) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::GainShield(1),
            on_failure: Action::GainArmor(12),
        }],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Warded Sabatons",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (1, 1), (1, 2)],
        base: Stats { armor: 9, mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::GainShield(1),
            on_failure: Action::GainArmor(9),
        }],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Ashfall Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Curse {
            kind: CurseKind::Searing,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 100,
        price: 6,
    },
    PieceDef {
        name: "Tidewrack Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainMana(2))],
        quest: None,
        power_bonus: 110,
        price: 9,
    },
    PieceDef {
        name: "Wrathwrit Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Rage, amount: 2 })],
        quest: None,
        power_bonus: 115,
        price: 10,
    },
    PieceDef {
        name: "Gravebloom Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Nature, amount: 2 })],
        quest: None,
        power_bonus: 120,
        price: 12,
    },
    PieceDef {
        name: "Oathbound Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain { what: Resource::Faith, amount: 2 })],
        quest: None,
        power_bonus: 120,
        price: 12,
    },
    PieceDef {
        name: "Mercurial Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 12,
        triggers: &[Trigger::OnActivate(Action::ReduceCooldown(200))],
        quest: None,
        power_bonus: 95,
        price: 8,
    },
    PieceDef {
        name: "Runewash Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Washes the runes off their gear for a while.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Misfire, target: Target::Enemy },
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 135,
        price: 14,
    },
    PieceDef {
        name: "Cinderscript Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Searing, target: Target::Enemy })],
        quest: None,
        power_bonus: 125,
        price: 15,
    },
    PieceDef {
        name: "Glacier Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Frost, target: Target::Enemy })],
        quest: None,
        power_bonus: 118,
        price: 15,
    },
    PieceDef {
        name: "Hollow Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Hollow: it is worth what is not there.
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::OnActivate(Action::MindDamage {
            amount: 2,
            target: Target::Enemy,
        }))],
        quest: None,
        power_bonus: 150,
        price: 18,
    },
    PieceDef {
        name: "Deepwater Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainMana(3))],
        quest: None,
        power_bonus: 145,
        price: 17,
    },
    PieceDef {
        name: "Starlit Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0),(1,1),(2,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Lines up with whatever shares its rows.
        triggers: &[Trigger::OnAlignedActivate(Action::Damage {
            amount: 7,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 160,
        price: 20,
    },
    PieceDef {
        name: "Emberdust Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { magic_damage: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Rage,
            cost: 6,
            on_success: Action::Damage {
                amount: 16,
                kind: DamageType::Magic,
                target: Target::Enemy,
            },
            on_failure: Action::Gain { what: Resource::Rage, amount: 2 },
        }],
        quest: None,
        power_bonus: 130,
        price: 16,
    },
    PieceDef {
        name: "Voidwritten Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { magic_pierce: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::MindDamage { amount: 4, target: Target::Enemy },
        }],
        quest: None,
        power_bonus: 185,
        price: 26,
    },
    PieceDef {
        name: "Kingsblood Ink",
        slot: SlotKind::Weapon,
        kind: PieceKind::Ink,
        cells: &[(0,0),(1,0),(2,0),(0,1),(2,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Expensive to write with, and it knows it.
        triggers: &[Trigger::SpendMana {
            cost: 6,
            on_success: Action::Damage {
                amount: 42,
                kind: DamageType::Magic,
                target: Target::Enemy,
            },
            on_failure: Action::Curse { kind: CurseKind::Searing, target: Target::Yourself },
        }],
        quest: None,
        power_bonus: 205,
        price: 34,
    },
    PieceDef {
        name: "Echo Sigil",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0)],
        base: Stats { magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::GainMana(3))],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Resonant Chord",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { magic_damage: 7, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Damage { amount: 6, kind: DamageType::Magic, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Attendant Flame",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats { magic_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Curse { kind: CurseKind::Searing, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    PieceDef {
        name: "Mirror Ward",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::GainArmor(9))],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Sympathetic Bloom",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { magic_damage: 4, regen: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Gain { what: Resource::Nature, amount: 2 })],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Choir of Ash",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { magic_damage: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::MindDamage { amount: 2, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 21,
    },
    PieceDef {
        name: "Rite of Answer",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(1,1),(2,1)],
        base: Stats { magic_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Gain { what: Resource::Faith, amount: 3 })],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Sunder",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { magic_damage: 15, magic_pierce: 35, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Misfire, target: Target::Enemy },
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    PieceDef {
        name: "Frostbind",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(0,1)],
        base: Stats { magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Frost, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Hollow Lance",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(0,1),(0,2),(0,3)],
        base: Stats { magic_damage: 21, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::OnActivate(Action::Damage {
            amount: 4,
            kind: DamageType::Magic,
            target: Target::Enemy,
        }))],
        quest: None,
        power_bonus: 0,
        price: 26,
    },
    PieceDef {
        name: "Verdant Surge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend { what: Resource::Nature, cost: 4, on_success: Action::GainMana(8), on_failure: Action::Gain { what: Resource::Nature, amount: 3 } }],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Blood Rite",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats { magic_damage: 7, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend { what: Resource::Rage, cost: 5, on_success: Action::Damage { amount: 22, kind: DamageType::Magic, target: Target::Enemy }, on_failure: Action::Gain { what: Resource::Rage, amount: 3 } }],
        quest: None,
        power_bonus: 0,
        price: 23,
    },
    PieceDef {
        name: "Sanctuary",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend { what: Resource::Faith, cost: 4, on_success: Action::GainShield(1), on_failure: Action::GainArmor(12) }],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Cometfall",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { magic_damage: 26, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // It lands on them either way.
        triggers: &[Trigger::SpendMana {
            cost: 5,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
        }],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    PieceDef {
        name: "Unmaking",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { mind: 4, magic_damage: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::MindDamage { amount: 3, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 25,
    },
    PieceDef {
        name: "Azure Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(1,0)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Open water. Every cell it is not touching gear is a cell it draws from.
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::OnActivate(Action::GainMana(1)))],
        quest: None,
        power_bonus: 70,
        price: 12,
    },
    PieceDef {
        name: "Crimson Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(1,0)],
        base: Stats { rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Each spell that goes off feeds the next.
        triggers: &[Trigger::OnOtherCast(Action::Gain { what: Resource::Rage, amount: 3 })],
        quest: None,
        power_bonus: 70,
        price: 12,
    },
    PieceDef {
        name: "Golden Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(0,1)],
        base: Stats { faith: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Room around the ball is where the light gets in.
        //
        // Four, not the ten this was first written with: the repeat charges
        // it once per open cell, so ten meant forty faith an activation and
        // the trigger could never once pay. It banked faith and took the
        // failure branch forever. Nothing said so until spending a hold pool
        // started costing what it actually costs.
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::Spend {
            what: Resource::Faith,
            cost: 4,
            on_success: Action::GainShield(1),
            on_failure: Action::Gain { what: Resource::Faith, amount: 2 },
        })],
        quest: None,
        power_bonus: 70,
        price: 12,
    },
    PieceDef {
        name: "Verdant Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(0,1)],
        base: Stats { nature: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Gain { what: Resource::Nature, amount: 3 })],
        quest: None,
        power_bonus: 70,
        price: 12,
    },
    PieceDef {
        name: "Tidal Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats { mana: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana { cost: 4, on_success: Action::GainEmpowerment(1), on_failure: Action::GainMana(2) }],
        quest: None,
        power_bonus: 90,
        price: 20,
    },
    PieceDef {
        name: "Ember Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { rage: 2, magic_damage: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Rage,
            cost: 8,
            on_success: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
            on_failure: Action::Gain { what: Resource::Rage, amount: 3 },
        }],
        quest: None,
        power_bonus: 85,
        price: 19,
    },
    PieceDef {
        name: "Pilgrim Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { faith: 2, armor: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Faith,
            cost: 12,
            on_success: Action::GainArmor(22),
            on_failure: Action::Gain { what: Resource::Faith, amount: 3 },
        }],
        quest: None,
        power_bonus: 80,
        price: 18,
    },
    PieceDef {
        name: "Rootwork Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { nature: 3, regen: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Growth banked while the fight runs, which is what nature is for.
        triggers: &[Trigger::Spend {
            what: Resource::Nature,
            cost: 12,
            on_success: Action::Grow(14),
            on_failure: Action::Gain { what: Resource::Nature, amount: 3 },
        }],
        quest: None,
        power_bonus: 80,
        price: 19,
    },
    PieceDef {
        name: "Prism Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 1, rage: 1, faith: 1, nature: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // It takes whatever light reaches it, from wherever there is room.
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::OnActivate(Action::Gain {
            what: Resource::Mana,
            amount: 1,
        }))],
        quest: None,
        power_bonus: 95,
        price: 24,
    },
    PieceDef {
        name: "Void Alignment",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(1,0),(1,1),(2,1)],
        base: Stats { magic_pierce: 25, mind: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // The hole in the ball: their gear forgets what it was doing.
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Curse { kind: CurseKind::Misfire, target: Target::Enemy },
            on_failure: Action::MindDamage { amount: 3, target: Target::Enemy },
        }],
        quest: None,
        power_bonus: 100,
        price: 27,
    },
    PieceDef {
        name: "Ash Haft",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { strength: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2400,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    PieceDef {
        name: "Corded Grip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { strength: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2300,
        speed_bonus: 4,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Ironbound Haft",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { strength: 7, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Duelist's Hilt",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1)],
        base: Stats { strength: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 1700,
        speed_bonus: 8,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Whipcord Hilt",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1)],
        base: Stats { strength: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 1500,
        speed_bonus: 14,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Warden's Haft",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2),(0,3)],
        base: Stats { strength: 9, health: 100, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Sunder Haft",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2),(0,3)],
        base: Stats { strength: 13, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2700,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 21,
    },
    PieceDef {
        name: "Twinned Grip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { strength: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2000,
        speed_bonus: 6,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Gravebound Haft",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { strength: 10, mind_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Kingmaker Hilt",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { strength: 16, power: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2500,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 32,
    },
    PieceDef {
        name: "Chipped Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0)],
        base: Stats { physical_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 5,
    },
    PieceDef {
        name: "Hooked Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0)],
        base: Stats { physical_damage: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Sawtooth Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0)],
        base: Stats { physical_damage: 15, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Bronze Fang",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { physical_damage: 9, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Iron Fang",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { physical_damage: 15, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Adamant Fang",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { physical_damage: 23, physical_pierce: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    PieceDef {
        name: "Witchglass Shard",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { magic_damage: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Voidglass Shard",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { magic_damage: 20, magic_pierce: 30, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    PieceDef {
        name: "Reaver's Bill",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0),(1,1),(2,1)],
        base: Stats { physical_damage: 18, physical_pierce: 15, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Worldsplitter",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { physical_damage: 30, physical_pierce: 35, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 38,
    },
    PieceDef {
        name: "Bone Charm",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0)],
        base: Stats { strength: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    PieceDef {
        name: "Silver Charm",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0)],
        base: Stats { strength: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Loaded Fob",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0)],
        base: Stats { power: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Duelist's Fob",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0)],
        base: Stats { power: 35, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Windup Key",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(1,0)],
        base: Stats { power: 15, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 14,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Clockwork Key",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(1,0)],
        base: Stats { power: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 22,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Ratchet Cog",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 26,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Flywheel Cog",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(0,1)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 38,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    PieceDef {
        name: "Bloodstone Bead",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(1,0)],
        base: Stats { rage: 2, physical_damage: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Oathstone Bead",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(1,0)],
        base: Stats { faith: 2, magic_resist: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Tin Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 6, health: 30, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Bronze Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 10, health: 70, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Warded Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 16, health: 110, magic_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Ridged Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { armor: 14, health: 90, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Buttressed Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { armor: 22, health: 150, physical_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Hollowbone Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { armor: 8, mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Ossuary Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { armor: 12, faith: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Stormcaught Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { armor: 26, health: 170, magic_resist: 14, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 26,
    },
    PieceDef {
        name: "Anvil Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { armor: 34, health: 220, physical_harden: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 32,
    },
    PieceDef {
        name: "Crown of Nails",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { armor: 11, physical_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Tin Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0)],
        base: Stats { armor: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Bronze Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0)],
        base: Stats { armor: 9, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Layered Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0)],
        base: Stats { armor: 14, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Scaled Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { armor: 11, physical_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Runed Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { armor: 11, magic_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Warded Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 17, magic_resist: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Bulwark Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 24, physical_harden: 18, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 23,
    },
    PieceDef {
        name: "Mirrorbright Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { armor: 13, magic_harden: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Deadweight Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { armor: 20, health: 90, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    PieceDef {
        name: "Godsteel Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { armor: 30, physical_resist: 14, magic_resist: 14, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 31,
    },
    PieceDef {
        name: "Feather Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0)],
        base: Stats { regen: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 4,
    },
    PieceDef {
        name: "Gilded Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0)],
        base: Stats { regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 8,
    },
    PieceDef {
        name: "Seer's Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // It sees the swing coming and they fumble it.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Misfire, target: Target::Enemy },
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Zealot's Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0)],
        base: Stats { faith: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Berserker's Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0)],
        base: Stats { rage: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Bloomed Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0)],
        base: Stats { nature: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Warlord's Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { strength: 6, rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Archon's Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { power: 30, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    PieceDef {
        name: "Martyr's Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { health: 200, regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Crown of the Deep",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { mana: 3, magic_pierce: 20, power: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::MindDamage { amount: 3, target: Target::Enemy },
        }],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    PieceDef {
        name: "Sackcloth Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 90, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Quilted Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 150, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Brigandine Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 220, armor: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Ribbed Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { health: 260, armor: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Bastion Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { health: 350, armor: 20, physical_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 27,
    },
    PieceDef {
        name: "Cinder Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { health: 170, rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Grove Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { health: 170, nature: 2, regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Chapel Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { health: 150, faith: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Wellspring Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { health: 130, mana: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Adamant Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { health: 440, armor: 26, magic_resist: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 34,
    },
    PieceDef {
        name: "Rag Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0)],
        base: Stats { armor: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Felt Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0)],
        base: Stats { armor: 11, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 6,
    },
    PieceDef {
        name: "Mail Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0)],
        base: Stats { armor: 17, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Scale Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { armor: 15, physical_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Sigil Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { armor: 15, magic_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Thorn Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { armor: 9, physical_damage: 7, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Mending Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { armor: 9, regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Bulwark Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 24, physical_harden: 16, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 21,
    },
    PieceDef {
        name: "Aether Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 20, mana: 2, magic_harden: 16, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::GainArmor(10),
        }],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    PieceDef {
        name: "Godsheet Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { armor: 34, health: 150, physical_resist: 12, magic_resist: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 33,
    },
    PieceDef {
        name: "Hide Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 50, strength: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Waxed Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 90, strength: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Scaled Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 130, strength: 5, physical_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Spun Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { mana: 2, power: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Sanctified Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { faith: 2, magic_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Ashwoven Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { rage: 2, physical_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Rootwoven Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { nature: 2, regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 13,
    },
    PieceDef {
        name: "Ironthread Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { health: 170, armor: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Duskweave Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { magic_pierce: 22, mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        // Dusk: they can no longer quite see what they are doing.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Misfire, target: Target::Enemy },
            on_failure: Action::GainMana(1),
        }],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    PieceDef {
        name: "Worldweave Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { health: 240, strength: 8, armor: 14, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    PieceDef {
        name: "Padded Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0)],
        base: Stats { strength: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Braced Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0)],
        base: Stats { strength: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Vicegrip Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0)],
        base: Stats { strength: 7, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Nimble Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { power: 15, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 12,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Quickfinger Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { power: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 20,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Warding Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 14, magic_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Rending Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { physical_damage: 11, physical_pierce: 18, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Oathkeeper Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { faith: 3, armor: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Wrathful Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { rage: 3, physical_damage: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Sovereign Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { strength: 11, power: 30, armor: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 29,
    },
    PieceDef {
        name: "Plain Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0)],
        base: Stats { regen: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Sprung Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0)],
        base: Stats { regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 8,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Racing Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0)],
        base: Stats { regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 18,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Anchored Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { health: 130, armor: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Trailworn Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { nature: 2, regen: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Pilgrim Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { faith: 3, magic_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 16,
    },
    PieceDef {
        name: "Ironshod Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 20, physical_harden: 14, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    PieceDef {
        name: "Stormstep Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { mana: 2, power: 18, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 10,
        // A step ahead, and they lose a beat.
        triggers: &[Trigger::SpendMana {
            cost: 3,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::ReduceCooldown(200),
        }],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    PieceDef {
        name: "Gravewalker Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { mind_resist: 20, health: 150, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Worldstrider Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { regen: 4, armor: 18, health: 200, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 6,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 31,
    },
    PieceDef {
        name: "Tin Band",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { strength: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 3,
    },
    PieceDef {
        name: "Silver Band",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { strength: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 7,
    },
    PieceDef {
        name: "Signet of Iron",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { physical_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Signet of Ash",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { magic_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Ring of Wells",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Ring of Embers",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Ring of Vigils",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { faith: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Ring of Roots",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { nature: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 10,
    },
    PieceDef {
        name: "Seal of Power",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0),(1,0)],
        base: Stats { power: 30, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Seal of the Deep",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0),(1,0)],
        base: Stats { mana: 3, magic_pierce: 20, power: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 27,
    },
    PieceDef {
        name: "Chapbook",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        // Short enough to read twice.
        triggers: &[Trigger::OnActivate(Action::ReduceCooldown(150))],
        quest: None,
        power_bonus: 0,
        price: 5,
    },
    PieceDef {
        name: "Traveller's Codex",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        // Read on the move: it answers whatever is keeping pace with it.
        triggers: &[Trigger::OnAlignedActivate(Action::GainMana(1))],
        quest: None,
        power_bonus: 0,
        price: 9,
    },
    PieceDef {
        name: "Scholar's Codex",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2200,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::GainMana(2))],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Hymnal",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { faith: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Faith,
            cost: 10,
            on_success: Action::GainArmor(18),
            on_failure: Action::Gain { what: Resource::Faith, amount: 3 },
        }],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "War Ledger",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        // It settles accounts.
        triggers: &[Trigger::Spend {
            what: Resource::Rage,
            cost: 10,
            on_success: Action::Damage {
                amount: 24,
                kind: DamageType::Physical,
                target: Target::Enemy,
            },
            on_failure: Action::Gain { what: Resource::Rage, amount: 3 },
        }],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Herbal",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { nature: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2800,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Nature,
            cost: 10,
            on_success: Action::Grow(11),
            on_failure: Action::Gain { what: Resource::Nature, amount: 3 },
        }],
        quest: None,
        power_bonus: 0,
        price: 12,
    },
    PieceDef {
        name: "Quickread Folio",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2000,
        speed_bonus: 12,
        triggers: &[Trigger::OnActivate(Action::ReduceCooldown(300))],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Whisperbound Tome",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { mana: 2, mind_resist: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2400,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::MindDamage { amount: 3, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Grand Grimoire",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { mana: 3, power: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Damage {
            amount: 8,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 28,
    },
    PieceDef {
        name: "Codex Interminable",
        slot: SlotKind::Weapon,
        kind: PieceKind::Book,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { mana: 4, power: 35, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2400,
        speed_bonus: 6,
        // It never ends, so it fills whatever room it is given.
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::OnActivate(Action::Damage {
            amount: 5,
            kind: DamageType::Magic,
            target: Target::Enemy,
        }))],
        quest: None,
        power_bonus: 0,
        price: 38,
    },
    PieceDef {
        name: "Clouded Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::MindDamage { amount: 2, target: Target::Enemy })],
        quest: None,
        power_bonus: 40,
        price: 7,
    },
    PieceDef {
        name: "Polished Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2900,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Damage {
            amount: 5,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 55,
        price: 12,
    },
    PieceDef {
        name: "Fateglass Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        // Every spell it holds shows them a future they then fumble.
        triggers: &[Trigger::OnOtherCast(Action::Curse {
            kind: CurseKind::Misfire,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 75,
        price: 19,
    },
    PieceDef {
        name: "Tidecaller Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { mana: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2700,
        speed_bonus: 0,
        // One spell pulls the next in behind it.
        triggers: &[Trigger::OnOtherCast(Action::GainMana(2))],
        quest: None,
        power_bonus: 75,
        price: 22,
    },
    PieceDef {
        name: "Emberheart Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { rage: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2700,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Rage,
            cost: 9,
            on_success: Action::Damage {
                amount: 26,
                kind: DamageType::Magic,
                target: Target::Enemy,
            },
            on_failure: Action::Gain { what: Resource::Rage, amount: 3 },
        }],
        quest: None,
        power_bonus: 65,
        price: 22,
    },
    PieceDef {
        name: "Grovemind Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { nature: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2700,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Nature,
            cost: 9,
            on_success: Action::Grow(16),
            on_failure: Action::Gain { what: Resource::Nature, amount: 3 },
        }],
        quest: None,
        power_bonus: 65,
        price: 22,
    },
    PieceDef {
        name: "Reliquary Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { faith: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2700,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Faith,
            cost: 9,
            on_success: Action::GainShield(1),
            on_failure: Action::Gain { what: Resource::Faith, amount: 3 },
        }],
        quest: None,
        power_bonus: 65,
        price: 22,
    },
    PieceDef {
        name: "Spinning Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2200,
        speed_bonus: 14,
        // Every spell that goes off gives it another shove.
        triggers: &[Trigger::OnOtherCast(Action::ReduceCooldown(180))],
        quest: None,
        power_bonus: 60,
        price: 26,
    },
    PieceDef {
        name: "Orb of the Nine",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { mana: 4, power: 25, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2500,
        speed_bonus: 0,
        // Nine of them, and each wants a window.
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::SpendMana {
            cost: 2,
            on_success: Action::Damage {
                amount: 11,
                kind: DamageType::Magic,
                target: Target::Enemy,
            },
            on_failure: Action::GainMana(1),
        })],
        quest: None,
        power_bonus: 95,
        price: 33,
    },
    PieceDef {
        name: "Worldeye Orb",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { mana: 5, power: 40, magic_pierce: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2300,
        speed_bonus: 8,
        // It looks at them and they stop.
        triggers: &[Trigger::OnOtherCast(Action::Curse {
            kind: CurseKind::Stun,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 120,
        price: 45,
    },
    // Francis only. Never stocked, and deliberately outside the scale every
    // other chestpiece is measured against - see BOSS_ONLY.
    PieceDef {
        name: "The Money Jacket",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(3,0),(0,1),(1,1),(2,1),(3,1),(0,2),(1,2),(2,2),(3,2)],
        base: Stats {
            health: 2100,
            armor: 90,
            regen: 9,
            strength: 26,
            physical_resist: 40,
            magic_resist: 40,
            physical_harden: 30,
            magic_harden: 30,
            mind_resist: 40,
            curse_resist: 40,
            ..Stats::ZERO
        },
        adjacency: None,
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        triggers: &[
            Trigger::OnActivate(Action::GainArmor(70)),
            Trigger::OnActivate(Action::Damage { amount: 40, kind: DamageType::Physical, target: Target::Enemy }),
        ],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    PieceDef {
        name: "Heartwood Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0)],
        base: Stats { health: 60, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Grow(40))],
        quest: None,
        power_bonus: 0,
        price: 46,
    },
    PieceDef {
        name: "The Growing Weight",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { health: 90, armor: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Grow(60))],
        quest: None,
        power_bonus: 0,
        price: 62,
    },
    PieceDef {
        name: "Grasping Ring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { health: 40, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Grow(30))],
        quest: None,
        power_bonus: 0,
        price: 38,
    },
    PieceDef {
        name: "Deeprooted Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { health: 70, regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Grow(45))],
        quest: None,
        power_bonus: 0,
        price: 52,
    },
    PieceDef {
        name: "Gluttonous Fang",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(0,1),(1,1)],
        base: Stats { physical_damage: 9, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Grow(35))],
        quest: None,
        power_bonus: 0,
        price: 55,
    },
    PieceDef {
        name: "Hermit's Band",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { health: 40, strength: 3, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Hermit: everything x6 while its row is its own", when: When::Assembled, kind: EffectKind::SoleIf { what: Solitude::Row, times: 6 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 58,
    },
    PieceDef {
        name: "The Empty Crown",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0)],
        base: Stats { health: 55, armor: 12, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Empty Crown: everything x5 while its row is its own", when: When::Assembled, kind: EffectKind::SoleIf { what: Solitude::Row, times: 5 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 64,
    },
    PieceDef {
        name: "Lonely Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0)],
        base: Stats { armor: 14, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Lonely: everything x4 while nothing overlaps it", when: When::Assembled, kind: EffectKind::SoleIf { what: Solitude::Stacked, times: 4 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 54,
    },
    PieceDef {
        name: "Widow's Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(0,1)],
        base: Stats { health: 45, regen: 2, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Widow: everything x4 while nothing overlaps it", when: When::Assembled, kind: EffectKind::SoleIf { what: Solitude::Stacked, times: 4 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 56,
    },
    PieceDef {
        name: "Bare-Headed Fang",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0)],
        base: Stats { physical_damage: 11, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Bare-Headed: everything x3 while no helmet overlaps it", when: When::Assembled, kind: EffectKind::SoleIf { what: Solitude::StackedWith(SlotKind::Helmet), times: 3 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 44,
    },
    PieceDef {
        name: "Ungloved Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { armor: 16, health: 50, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Ungloved: everything x3 while no glove overlaps it", when: When::Assembled, kind: EffectKind::SoleIf { what: Solitude::StackedWith(SlotKind::Gloves), times: 3 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 46,
    },
    PieceDef {
        name: "Unshod Signet",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { magic_damage: 7, mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: Some(Effect { label: "Unshod: everything x10 while no greave overlaps it", when: When::Assembled, kind: EffectKind::SoleIf { what: Solitude::StackedWith(SlotKind::Greaves), times: 10 } }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAlignedActivate(Action::Curse {
            kind: CurseKind::Misfire,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 88,
    },

    // ---- spending a pool, rather than only banking one ---------------------
    //
    // A survey of the catalogue found every sink was a fixed threshold, and
    // that each pool could buy exactly one kind of thing: faith only bought
    // defence, nature only bought health, rage only bought damage, and mana
    // never bought growth at all. Sixteen pieces against that, plus the first
    // sinks outside the weapon slot that any of the hold pools have had.
    // Faith kept is a wall. Faith spent all at once is a verdict.
    PieceDef {
        name: "Reckoning Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0)],
        base: Stats { faith: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Faith,
            each: 6,
            per: Action::Damage {
                amount: 11,
                kind: DamageType::Magic,
                target: Target::Enemy,
            },
        }],
        quest: None,
        power_bonus: 0,
        price: 26,
    },
    PieceDef {
        name: "Zealot's Haft",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { faith: 2, strength: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2600,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Faith,
            cost: 7,
            on_success: Action::Damage {
                amount: 19,
                kind: DamageType::Physical,
                target: Target::Enemy,
            },
            on_failure: Action::Gain { what: Resource::Faith, amount: 3 },
        }],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    // Everything that grows has thorns on it somewhere.
    PieceDef {
        name: "Bramble Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { nature: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Nature,
            cost: 5,
            on_success: Action::Damage {
                amount: 21,
                kind: DamageType::Physical,
                target: Target::Enemy,
            },
            on_failure: Action::Gain { what: Resource::Nature, amount: 3 },
        }],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Wildfire Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { nature: 2, ..Stats::health(40) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Nature,
            each: 5,
            per: Action::Damage {
                amount: 8,
                kind: DamageType::Magic,
                target: Target::Enemy,
            },
        }],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    // Fury spent on staying upright, which is not what fury is for.
    PieceDef {
        name: "Scarred Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0)],
        base: Stats { rage: 2, ..Stats::armor(6) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Rage,
            cost: 6,
            on_success: Action::GainArmor(30),
            on_failure: Action::Gain { what: Resource::Rage, amount: 3 },
        }],
        quest: None,
        power_bonus: 0,
        price: 21,
    },
    PieceDef {
        name: "Bloodbank Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { rage: 2, ..Stats::health(60) },
        adjacency: None,
        effect: None,
        cooldown_ms: 4800,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Rage,
            each: 6,
            per: Action::GainArmor(11),
        }],
        quest: None,
        power_bonus: 0,
        price: 27,
    },
    PieceDef {
        name: "Wellspring Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Mana,
            cost: 4,
            on_success: Action::Grow(15),
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 0,
        price: 23,
    },
    // Drink the whole reserve. You keep what it makes of you.
    PieceDef {
        name: "Deepdraught Ring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Mana,
            each: 5,
            per: Action::Grow(9),
        }],
        quest: None,
        power_bonus: 0,
        price: 19,
    },
    // Conviction, cashed in for something less patient.
    PieceDef {
        name: "Tithe Ring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { faith: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Faith,
            cost: 5,
            on_success: Action::Gain { what: Resource::Rage, amount: 8 },
            on_failure: Action::Gain { what: Resource::Faith, amount: 2 },
        }],
        quest: None,
        power_bonus: 0,
        price: 17,
    },
    // What burns down feeds what grows back.
    PieceDef {
        name: "Ashen Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { rage: 2, ..Stats::health(35) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Rage,
            cost: 5,
            on_success: Action::Gain { what: Resource::Nature, amount: 8 },
            on_failure: Action::Gain { what: Resource::Rage, amount: 2 },
        }],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Covenant Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(1,1)],
        base: Stats { mana: 2, ..Stats::health(45) },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Mana,
            cost: 4,
            on_success: Action::Gain { what: Resource::Faith, amount: 7 },
            on_failure: Action::GainMana(2),
        }],
        quest: None,
        power_bonus: 0,
        price: 20,
    },
    PieceDef {
        name: "Reliquary Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { faith: 2, ..Stats::armor(8) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Faith,
            each: 9,
            per: Action::Grow(12),
        }],
        quest: None,
        power_bonus: 0,
        price: 28,
    },
    // It has been keeping a list.
    PieceDef {
        name: "Grudge Bead",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0)],
        base: Stats { rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Rage,
            each: 5,
            per: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
        }],
        quest: None,
        power_bonus: 0,
        price: 25,
    },
    // Everything at once, and nothing left in the field.
    PieceDef {
        name: "Harvest Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(0,1)],
        base: Stats { nature: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Nature,
            each: 6,
            per: Action::Grow(13),
        }],
        quest: None,
        power_bonus: 0,
        price: 26,
    },
    // Conviction stops turning aside harm at forty percent. This is where the rest of it goes.
    PieceDef {
        name: "Overflow Plate",
        slot: SlotKind::Greaves,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { faith: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Faith,
            each: 8,
            per: Action::GainArmor(16),
        }],
        quest: None,
        power_bonus: 0,
        price: 29,
    },
    PieceDef {
        name: "Last Rite",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats { magic_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Faith,
            each: 4,
            per: Action::Damage {
                amount: 13,
                kind: DamageType::Magic,
                target: Target::Enemy,
            },
        }],
        quest: None,
        power_bonus: 0,
        price: 24,
    },

    // ---- trophies -----------------------------------------------------------
    //
    // One per named fight, dropped by the thing that was wearing it. All of it
    // is BOSS_ONLY: off the scale for its slot on purpose, kept out of the
    // shop, out of the slot ceiling, and out of the absurdity check.
    // It asks. You answer whether you meant to or not.
    PieceDef {
        name: "Asker's Monocle",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0)],
        base: Stats { mind: 26, mind_resist: 45, mana: 7, magic_pierce: 30, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::MindDamage { amount: 18, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    PieceDef {
        name: "Toolwright's Grip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2),(0,3)],
        base: Stats { strength: 30, physical_pierce: 30, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 1500,
        speed_bonus: 40,
        triggers: &[Trigger::OnActivate(Action::ReduceCooldown(400))],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    // Everything you build is, it turns out, a licensing matter.
    PieceDef {
        name: "Kaklon's Patent",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0),(0,1)],
        base: Stats { power: 90, mana: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 25,
        triggers: &[Trigger::OnOtherCast(Action::GainEmpowerment(1))],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    PieceDef {
        name: "Eighth Ray Crown",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0,0),(1,0),(2,0),(0,1),(2,1),(0,2),(2,2)],
        base: Stats { health: 900, faith: 6, magic_resist: 34, armor: 40, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3600,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Faith,
            each: 5,
            per: Action::GainShield(1),
        }],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    // Summoned by a claim nobody checked.
    PieceDef {
        name: "Assassin's Hemline",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0),(3,0)],
        base: Stats { physical_pierce: 45, magic_pierce: 45, strength: 22, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 60,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Misfire, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    // The oldest goof there is, and it has never once failed.
    PieceDef {
        name: "Handman's Peel",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0,0),(1,0),(1,1)],
        base: Stats { magic_damage: 88, magic_pierce: 55, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse { kind: CurseKind::Stun, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    // Not the jacket. The offcuts, which are still worth more than you are.
    PieceDef {
        name: "Gilded Offcuts",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1),(0,2),(1,2)],
        base: Stats { health: 1200, armor: 60, physical_resist: 34, magic_resist: 34, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainArmor(48))],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    // They do not fit anything. You keep them anyway.
    PieceDef {
        name: "Henpeck's Cell Keys",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { strength: 52, mana: 12, curse_resist: 60, physical_pierce: 45, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Curse { kind: CurseKind::Stun, target: Target::Enemy },
            on_failure: Action::GainMana(3),
        }],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    // He was looking for something. He is still looking.
    PieceDef {
        name: "The Seeker's Tears",
        slot: SlotKind::Weapon,
        kind: PieceKind::Orb,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 14, magic_damage: 38, magic_pierce: 45, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2000,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::Damage {
            amount: 40,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    // He is adrift on it between the planes, and cannot ascend while Francis lives.
    PieceDef {
        name: "Tetrahedron Shard",
        slot: SlotKind::Weapon,
        kind: PieceKind::Alignment,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { mana: 7, rage: 4, faith: 4, nature: 4, mind: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::PerAdjacentEmpty(&Trigger::OnActivate(Action::Damage {
            amount: 14,
            kind: DamageType::Magic,
            target: Target::Enemy,
        }))],
        quest: None,
        power_bonus: 0,
        price: 999,
    },

    // ---- what you walk in holding -------------------------------------------
    //
    // Armour and all four pools start every fight at zero, so the opening
    // seconds look the same whatever is on the board. These do not: one per
    // slot per resource family, so no build is shut out of an opening.
    // Braced before the bell. Everything else in the game starts at nothing.
    PieceDef {
        name: "Braced Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { armor: 8, ..Stats::health(60) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::GainArmor(90))],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    PieceDef {
        name: "Standing Start",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::GainMana(9))],
        quest: None,
        power_bonus: 0,
        price: 26,
    },
    // It arrives having already decided.
    PieceDef {
        name: "Opening Grudge",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0,0)],
        base: Stats { rage: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::Gain { what: Resource::Rage, amount: 14 })],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    PieceDef {
        name: "Vigil Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(0,1)],
        base: Stats { faith: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::Gain { what: Resource::Faith, amount: 14 })],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    PieceDef {
        name: "Seedbed Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { nature: 1, ..Stats::health(45) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::Gain { what: Resource::Nature, amount: 14 })],
        quest: None,
        power_bonus: 0,
        price: 25,
    },
    // Said before anyone is ready, which is most of why it lands.
    PieceDef {
        name: "First Word",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0)],
        base: Stats { magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::Damage {
            amount: 34,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 27,
    },
    // Two thousand miles an hour, and invisible with it.
    PieceDef {
        name: "Ambusher's Grip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Handle,
        cells: &[(0,0),(0,1),(0,2)],
        base: Stats { strength: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2400,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::Curse { kind: CurseKind::Stun, target: Target::Enemy })],
        quest: None,
        power_bonus: 0,
        price: 32,
    },
    PieceDef {
        name: "Bulwark Bead",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::GainShield(1))],
        quest: None,
        power_bonus: 0,
        price: 28,
    },
    PieceDef {
        name: "Warmed Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0,0),(1,0),(0,1),(1,1)],
        base: Stats { strength: 4, ..Stats::health(50) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::GainEmpowerment(1))],
        quest: None,
        power_bonus: 0,
        price: 31,
    },
    // Already grown by the time anyone swings.
    PieceDef {
        name: "Deep Roots Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)],
        base: Stats { nature: 2, ..Stats::health(180) },
        adjacency: None,
        effect: None,
        cooldown_ms: 5000,
        speed_bonus: 0,
        triggers: &[Trigger::OnBattleStart(Action::Grow(40))],
        quest: None,
        power_bonus: 0,
        price: 34,
    },

    // What the Dreaming Idiot leaves behind. Its whole trick, in a helmet.
    PieceDef {
        name: "The Idiot's Gift",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats {
            mind: 30,
            mind_resist: 55,
            nature: 6,
            regen: 8,
            ..Stats::ZERO
        },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[
            Trigger::OnBattleStart(Action::GainArmor(140)),
            Trigger::Consume {
                what: Resource::Nature,
                each: 6,
                per: Action::MindDamage { amount: 9, target: Target::Enemy },
            },
        ],
        quest: None,
        power_bonus: 0,
        price: 999,
    },

    // ---- spell forking ------------------------------------------------------
    //
    // A fork copies a cast, and only a cast - a blade swings once however many
    // stacks are up. One per slot, each spending a different pool, so a caster
    // can reach it from whatever their build already banks.
    // Devotion, spent on saying it twice.
    PieceDef {
        name: "Forked Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0,0),(1,0)],
        base: Stats { faith: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Faith,
            cost: 14,
            on_success: Action::GainForking(1),
            on_failure: Action::Gain { what: Resource::Faith, amount: 4 },
        }],
        quest: None,
        power_bonus: 0,
        price: 40,
    },
    // Everything that grows, grows in two directions.
    PieceDef {
        name: "Split Weave",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { nature: 2, ..Stats::health(40) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Nature,
            cost: 14,
            on_success: Action::GainForking(1),
            on_failure: Action::Gain { what: Resource::Nature, amount: 4 },
        }],
        quest: None,
        power_bonus: 0,
        price: 40,
    },
    PieceDef {
        name: "Twinning Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(0,1)],
        base: Stats { mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 8,
            on_success: Action::GainForking(1),
            on_failure: Action::GainMana(3),
        }],
        quest: None,
        power_bonus: 0,
        price: 42,
    },
    PieceDef {
        name: "Echo Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { rage: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Spend {
            what: Resource::Rage,
            cost: 14,
            on_success: Action::GainForking(1),
            on_failure: Action::Gain { what: Resource::Rage, amount: 4 },
        }],
        quest: None,
        power_bonus: 0,
        price: 40,
    },
    // Empty the reserve and every spell in the build says itself again.
    PieceDef {
        name: "Forking Bead",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0,0)],
        base: Stats { mana: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Mana,
            each: 10,
            per: Action::GainForking(1),
        }],
        quest: None,
        power_bonus: 0,
        price: 44,
    },

    // What the old gods were holding, split into pieces on the way out.
    PieceDef {
        name: "The Split Wisdom",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0), (1, 0)],
        base: Stats { power: 90, mana: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnOtherCast(Action::GainForking(1))],
        quest: None,
        power_bonus: 0,
        price: 999,
    },
    // The aimed stun. Shares Cometfall's footprint and kind on purpose: the
    // two are the same spell with and without a choice of target, so the
    // difficulty stepper can swap one for the other.
    PieceDef {
        name: "Kingsbane",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0, 0), (1, 0), (2, 0), (1, 1)],
        base: Stats { magic_damage: 18, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // One of the handful of stuns the weapon keeps. The nine mana it used
        // to want bought the *aiming* - picking which of their items to stop -
        // and aiming is the hands' trick now, so there is nothing left to pay
        // for. What remains is the plain unaimed curse a blade can manage:
        // it stops something, and it does not get to choose what.
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Stun,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 34,
    },
    // ---- taking a pool off them, one per slot -------------------------
    //
    // Every one of these is dead against a build that banks nothing, and a
    // build that banks nothing is most of the early ladder. They are answers
    // to a specific problem - the deep-pool caster, the rage engine - which is
    // why they sit in each slot's optional third kind rather than competing
    // with the pieces that hold a recipe together.
    PieceDef {
        name: "Leech Bead",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0)],
        base: Stats { magic_damage: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Draining is the hands' vocabulary now. What is left is a small
        // magic bead, and it is priced like one.
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 21,
    },
    PieceDef {
        name: "Doubter's Crest",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0), (1, 0)],
        base: Stats { health: 30, curse_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Drain {
            what: Resource::Faith,
            amount: 3,
            hurt: 0,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 23,
    },
    PieceDef {
        name: "Becalming Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (0, 1), (1, 1)],
        base: Stats { health: 55, physical_resist: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Drain {
            what: Resource::Rage,
            amount: 3,
            hurt: 0,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 24,
    },
    PieceDef {
        name: "Blightfinger",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0, 0)],
        base: Stats { magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Drain {
            what: Resource::Nature,
            amount: 3,
            hurt: 0,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 22,
    },
    PieceDef {
        name: "Sump Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (1, 1)],
        base: Stats { health: 34, mana: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Takes the lot rather than a slice, which is worth nothing against a
        // dry pool and decides a fight against a caster who has been saving.
        triggers: &[Trigger::OnActivate(Action::Drain {
            what: Resource::Mana,
            amount: 0,
            hurt: 0,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    // ---- and the same trick turned on the player ----------------------
    //
    // These take the whole pool and charge for it. Against a build that banks
    // nothing they are a blank; against one that has been saving for a big
    // spend they are the reason it never gets to make it. Ten creatures carry
    // one, which is enough that a hoarding build meets the answer without
    // every fight being about it.
    PieceDef {
        name: "Tithe Collector",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0), (1, 0)],
        base: Stats { health: 38, magic_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Drain {
            what: Resource::Faith,
            amount: 0,
            hurt: 3,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 38,
    },
    PieceDef {
        name: "Wrathbreaker",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats { health: 62, physical_resist: 7, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Drain {
            what: Resource::Rage,
            amount: 0,
            hurt: 3,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 40,
    },
    PieceDef {
        name: "Witherroot",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (1, 1)],
        base: Stats { health: 40, magic_damage: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Drain {
            what: Resource::Nature,
            amount: 0,
            hurt: 3,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 39,
    },
    PieceDef {
        name: "Manaflay",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0), (1, 0)],
        base: Stats { magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // It used to strip a pool and charge two damage a point for it. That
        // is a glove's job now; what is left is the blade without the theft.
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    // ---- the casino chips -------------------------------------------
    //
    // Neither is buyable. They come out of the casino or they do not come at
    // all, which is why they are exempt from the shop - see `EVENT_ONLY`.
    PieceDef {
        // `price` is vestigial - `shop_price` derives cost from the rating -
        // but it may not be zero, and neither chip is on a shelf anyway.
        name: "Gold Chip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0)],
        base: Stats { magic_damage: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Five fnorp a swing, hitting four harder every time it pays, and it
        // stops at forty - so the worst it can do to your shopping is known
        // before you put it on. Both the budget and the escalation reset when
        // the next fight starts.
        triggers: &[Trigger::SpendGold {
            cost: 5,
            budget: 40,
            on_success: Action::Damage {
                amount: 4,
                kind: crate::combat::DamageType::Magic,
                target: Target::Enemy,
            },
        }],
        quest: None,
        power_bonus: 0,
        price: 1,
    },
    PieceDef {
        name: "Platinum Chip",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0)],
        // Barely a component. It is a key, and it costs you a cell to keep -
        // which is the whole cost of holding on to it until rung thirty.
        base: Stats { magic_damage: 2, mana: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 1,
    },
    // ---- behind the velvet rope --------------------------------------
    //
    // Five things worth more than anything on an honest shelf, which is the
    // point: what they cost is not gold. They are exempt from `slot_ceiling`
    // (see VIP_ONLY), so their numbers do not deflate the price of ordinary
    // gear in the same slots.
    PieceDef {
        name: "Overseer's Circlet",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats {
            health: 210,
            physical_resist: 26,
            magic_resist: 26,
            mind_resist: 30,
            ..Stats::ZERO
        },
        adjacency: None,
        effect: None,
        cooldown_ms: 3200,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::GainArmor(40))],
        quest: None,
        power_bonus: 0,
        price: 480,
    },
    PieceDef {
        name: "Foreman's Harness",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)],
        base: Stats { health: 420, physical_resist: 20, physical_harden: 30, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Grow(18))],
        quest: None,
        power_bonus: 0,
        price: 520,
    },
    PieceDef {
        name: "Tallykeeper's Weave",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { health: 120, mana: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 2000,
        speed_bonus: 30,
        triggers: &[Trigger::OnActivate(Action::GainForking(1))],
        quest: None,
        power_bonus: 0,
        price: 500,
    },
    PieceDef {
        name: "Treadmill Sole",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (2, 0), (1, 1)],
        base: Stats { health: 150, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        // A mold is not a core, so it has no cadence of its own to set.
        cooldown_ms: 0,
        speed_bonus: 25,
        triggers: &[Trigger::OnActivate(Action::ReduceCooldown(400))],
        quest: None,
        power_bonus: 0,
        price: 470,
    },
    PieceDef {
        name: "Quota Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats { physical_damage: 88, physical_pierce: 45, strength: 20, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::MindDamage {
            amount: 6,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 560,
    },
    // ---- town gear ----
    //
    // Sold three times a run at most, and *not* exempt from the rating scale
    // the way the VIP shelves are. The VIP shop is behind a locked branch and
    // its five pieces are meant to be absurd; a town is on the way to
    // everywhere, and five outliers three times a run would flatten the whole
    // curve. What makes these worth the trip is shape and effect, not size.
    PieceDef {
        // A helmet frame with a hole in it, so a spell can sit in its middle -
        // the one thing the ordinary helmet frames never let you do.
        name: "Lamplighter's Cage",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        base: Stats { health: 165, faith: 3, mind_resist: 8, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 3400,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 90,
    },
    PieceDef {
        // One cell. There is nothing else in the game that fits in a gap this
        // small, which is exactly what a tightly packed board runs out of.
        name: "Wickstub",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0)],
        base: Stats { health: 55, armor: 9, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 70,
    },
    PieceDef {
        // Pays out on being hit rather than on hitting, which is the half of
        // the game the ordinary glove stock hardly touches.
        name: "Toll-Taker's Mitt",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0, 0), (1, 0), (1, 1)],
        base: Stats { health: 70, curse_resist: 12, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain {
            what: Resource::Mana,
            amount: 4,
        })],
        quest: None,
        power_bonus: 0,
        price: 110,
    },
    PieceDef {
        // A long thin sole. Greaves boards are wide and shallow once a couple
        // of blocks are in; this is the piece that goes along the bottom.
        name: "Ridge Runner",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats { health: 90, armor: 12, nature: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 8,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 130,
    },
    PieceDef {
        // Cheap, fast, and worth having only if the rest of the board is fast
        // too - the opposite argument to everything else on the weapon shelf.
        name: "Kettleworks Pin",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1)],
        base: Stats { physical_damage: 26, strength: 6, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 22,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 120,
    },
    // ---- what the two rumour doors hand over ----
    //
    // Not off the scale: you paid for these with a component and a condition,
    // not with a locked branch, and the condition is the interesting part.
    PieceDef {
        name: "Crownwright's Measure",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0), (1, 0)],
        base: Stats { health: 120, mind_resist: 14, faith: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain {
            what: Resource::Faith,
            amount: 5,
        })],
        quest: None,
        power_bonus: 0,
        price: 200,
    },
    PieceDef {
        name: "The Green Ledger",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { health: 240, nature: 8, curse_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Gain {
            what: Resource::Nature,
            amount: 9,
        })],
        quest: None,
        power_bonus: 0,
        price: 260,
    },
    // ---- rumours ----
    //
    // One cell and nothing on it. A rumour is a component so that it can be
    // held, sold and bartered like anything else, but seating one costs a cell
    // and gains nothing - it is not gear, it is a condition. See `rumour.rs`.
    PieceDef {
        name: "A Word About the Crownwright",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 1,
    },
    PieceDef {
        name: "A Word About the Green Ledger",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 1,
    },
    // Not for sale, and not exempt from anything: what it is worth is the
    // thirty cells it hands you, and those are not on its card.
    PieceDef {
        name: "Sprocketman's Gratitude",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0)],
        base: Stats { health: 60, curse_resist: 10, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 1,
    },
    // ---- appended, and appended on purpose ----
    //
    // CATALOG is a wire format. A share code writes a component down as its
    // *position* here, so inserting one anywhere but the end re-points every
    // board anybody has saved - silently, because the code still reads, it
    // just comes back as somebody else's gear. Both finished runs in `share`
    // were decoded into nonsense by putting one piece in the middle of this
    // list. Append. Never insert.
    PieceDef {
        // Everything you have grown, set alight at once. A nature build banks
        // steadily all fight and has nowhere to spend it; this is the sink,
        // and it pays in a curse that stacks without a ceiling, so what it is
        // worth is exactly how patient the board has been.
        name: "Slash and Burn",
        slot: SlotKind::Weapon,
        kind: PieceKind::Spell,
        cells: &[(0,0),(1,0),(2,0)],
        base: Stats { magic_damage: 8, nature: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::Consume {
            what: Resource::Nature,
            each: 8,
            per: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
        }],
        quest: None,
        power_bonus: 0,
        price: 26,
    },
    // Stands on the bar beside the rumours and is traded for the same way,
    // but what it hands over is a class rather than a condition. It never
    // reaches the tray: `Run::barter` turns it into a stack of Recycler.
    PieceDef {
        name: "Scrap Ticket",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 1,
    },
    // ---- The hands, answering ----
    //
    // Where the weapon's reaction and denial games went. Every one of these
    // carries a mechanic that used to sit on a weapon piece, at the number it
    // sat there with: moving a monopoly means the game keeps the mechanic and
    // the weapon stops being the only place to find it.
    //
    // On rings and molds, never on a material. A material is one of the two
    // kinds `PieceDef::fits` lets cross into another grid, so anything
    // identity-carrying on one would be a gloves mechanic sitting in a greaves
    // board - which is the whole reason the floating kinds carry no identity.
    PieceDef {
        name: "Answering Ring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0, 0)],
        base: Stats { strength: 1, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // The hands answer. The smallest possible statement of it.
        triggers: &[Trigger::OnAdjacentActivate(Action::Damage {
            amount: 3,
            kind: DamageType::Physical,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 11,
    },
    PieceDef {
        name: "Mirrorplate Ring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0, 0), (1, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Mirrorcast's answer, on the hand it belonged on.
        triggers: &[Trigger::OnAdjacentActivate(Action::Damage {
            amount: 7,
            kind: DamageType::Magic,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 14,
    },
    PieceDef {
        name: "Chainlink Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (0, 1), (1, 1)],
        base: Stats { strength: 2, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAdjacentActivate(Action::Damage {
            amount: 5,
            kind: DamageType::Physical,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 15,
    },
    PieceDef {
        name: "Storm Signet",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0, 0), (1, 0)],
        base: Stats::ZERO,
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Arc Lightning's jump, counted the hands' way: once for every
        // finished item standing anywhere on the five boards.
        triggers: &[Trigger::PerAdjacentItem {
            action: Action::Damage { amount: 6, kind: DamageType::Magic, target: Target::Enemy },
            same_slot_only: false,
        }],
        quest: None,
        power_bonus: 0,
        price: 18,
    },
    PieceDef {
        name: "Siphon Ring",
        slot: SlotKind::Gloves,
        kind: PieceKind::Ring,
        cells: &[(0, 0)],
        base: Stats { magic_damage: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAdjacentActivate(Action::Drain {
            what: Resource::Mana,
            amount: 4,
            hurt: 0,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 21,
    },
    PieceDef {
        name: "Flaying Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats { magic_damage: 5, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Manaflay's trick: it takes the lot and charges for every point of
        // it, so it is worth nothing against an empty pool and a great deal
        // against a build that hoards.
        triggers: &[Trigger::OnAlignedActivate(Action::Drain {
            what: Resource::Mana,
            amount: 0,
            hurt: 2,
            target: Target::Enemy,
        })],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
    PieceDef {
        name: "Throttling Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { strength: 3, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Choosing which item to stop is what a hand can do and a blade
        // cannot. Pay for the aim; run dry and it takes whatever it catches.
        triggers: &[Trigger::SpendMana {
            cost: 9,
            on_success: Action::StunStrongest { target: Target::Enemy },
            on_failure: Action::Drain { what: Resource::Mana, amount: 2, hurt: 0, target: Target::Enemy },
        }],
        quest: None,
        power_bonus: 0,
        price: 34,
    },
    // ---- Terrain ----
    //
    // The first underlay. Laid under a grid rather than packed into it: gear
    // may stand on top of it, and what it is worth is decided by what does.
    //
    // Appended, and appended is the only way a component may ever join this
    // list - a share code stores a piece as its position here, so inserting
    // one anywhere else silently re-points every board anybody has saved.
    //
    // It is here in the pull request that built the underlay layer rather than
    // in the chest sweep that will use it, because a mechanic with nothing
    // carrying it cannot be tested, and shipping placement rules that nothing
    // has ever exercised is how they turn out to be wrong later.
    PieceDef {
        name: "Keystone Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Terrain,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::health(10),
        adjacency: None,
        effect: Some(Effect {
            label: "for each item built on top of it",
            kind: EffectKind::PerOverlappingCore { stat: StatKind::Power, amount: 10 },
            when: When::Always,
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        quest: None,
        power_bonus: 0,
        price: 30,
    },
];

/// Gear that exists only on a boss.
///
/// Kept out of the shop, and out of the scale every other piece is rated
/// against. One absurd chestpiece in the ceiling would quietly deflate the
/// rating - and so the rarity mark and the price - of every other chestpiece
/// in the game.
pub const BOSS_ONLY: &[&str] = &["The Money Jacket", "The Split Wisdom", "The Idiot's Gift", "Asker's Monocle", "Toolwright's Grip", "Kaklon's Patent", "Eighth Ray Crown", "Assassin's Hemline", "Handman's Peel", "Gilded Offcuts", "Henpeck's Cell Keys", "The Seeker's Tears", "Tetrahedron Shard"];

/// Is this a piece a player can never own?
pub fn is_boss_only(name: &str) -> bool {
    BOSS_ONLY.contains(&name)
}

/// Gear that only an event hands out.
///
/// Not boss gear: a player can absolutely own these, and is meant to. They are
/// simply not for sale, because what they are worth is the story of how you
/// got them - a Platinum Chip bought off a shelf is a door key with no door
/// behind it.
pub const EVENT_ONLY: &[&str] = &[
    "Gold Chip",
    "Platinum Chip",
    "Sprocketman's Gratitude",
    // Rumours are bartered for, never bought. See `rumour.rs`.
    "A Word About the Crownwright",
    "A Word About the Green Ledger",
    // And what the doors they open hand over.
    "Crownwright's Measure",
    "The Green Ledger",
    // Traded for a boss trophy at a pub, and never anything else.
    "Scrap Ticket",
];

/// The five things on the shelves behind the velvet rope.
///
/// Off the scale on purpose, and therefore exempt from it: `slot_ceiling` is
/// the best possible item in a slot and every ordinary rating is a fraction of
/// it, so five outliers left in the reckoning would deflate the price of
/// everything else in those slots. Same exemption `BOSS_ONLY` gets, for the
/// same reason - but these are not boss gear, because you buy them.
pub const VIP_ONLY: &[&str] = &[
    "Overseer's Circlet",
    "Foreman's Harness",
    "Tallykeeper's Weave",
    "Treadmill Sole",
    "Quota Edge",
];

pub fn is_vip_only(name: &str) -> bool {
    VIP_ONLY.contains(&name)
}

/// The five shelves a town's shop puts out.
///
/// Deliberately *not* off the scale. The VIP five are behind a locked branch
/// and are meant to be absurd; a town is on the way to everywhere, and five
/// outliers three times a run would flatten the whole curve. These earn their
/// place with shapes and effects the ordinary shop does not stock - a frame
/// with a hole in it, a single cell, a four-long sole - which is worth more to
/// a full board than another large number would be.
pub const TOWN_ONLY: &[&str] = &[
    "Lamplighter's Cage",
    "Wickstub",
    "Toll-Taker's Mitt",
    "Ridge Runner",
    "Kettleworks Pin",
];

pub fn is_town_only(name: &str) -> bool {
    TOWN_ONLY.contains(&name)
}

/// Is this piece kept out of the reckoning that prices everything else?
pub fn is_off_the_scale(name: &str) -> bool {
    is_boss_only(name) || is_vip_only(name)
}

pub fn is_event_only(name: &str) -> bool {
    EVENT_ONLY.contains(&name)
}

/// Is this piece the far side of somebody's quest?
///
/// A quest reward has to be earned. Finding one on a shelf makes the quest
/// that leads to it pointless - you would just buy the answer - so these are
/// kept off the shelves entirely and exist only as something a piece turns
/// into.
pub fn is_quest_reward(name: &str) -> bool {
    CATALOG.iter().any(|d| d.quest.is_some_and(|q| q.becomes == name))
}

/// Index of every definition in `CATALOG`, in catalog order.
pub fn all_def_indices() -> Vec<usize> {
    (0..CATALOG.len()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_slot_has_something_that_rewards_assembling_it() {
        // Not "exactly one" any more — several pieces carry assembly bonuses
        // now. What still has to hold is that no slot is left without a reason
        // to finish its gear.
        for slot in SlotKind::ALL {
            let n = CATALOG
                .iter()
                .filter(|d| d.slot == slot && d.adjacency.is_some())
                .count();
            assert!(n >= 1, "{} has no piece that pays off on assembly", slot.name());
        }
    }

    #[test]
    fn every_piece_is_priced_and_shaped() {
        for d in CATALOG {
            assert!(d.price > 0, "{} is free", d.name);
            assert!(!d.cells.is_empty(), "{} has no shape", d.name);
        }
    }

    #[test]
    fn a_core_piece_always_names_a_cooldown_path() {
        // Non-core pieces must not carry a cooldown: it would be silently
        // ignored, since only the core's timing is used.
        for d in CATALOG {
            if !d.kind.is_core() {
                assert_eq!(d.cooldown_ms, 0, "{} sets a cooldown it cannot use", d.name);
            }
        }
    }

    #[test]
    fn registry_rotation_cycles_and_changes_the_shape() {
        let mut reg = PieceRegistry::new();
        let ell = CATALOG.iter().position(|d| d.name == "Gauntlet Mold").unwrap();
        let id = reg.alloc(ell);

        let original = reg.shape(id);
        reg.rotate_cw(id);
        assert_ne!(reg.shape(id), original);
        for _ in 0..3 {
            reg.rotate_cw(id);
        }
        assert_eq!(reg.shape(id), original, "four turns returns to start");
        assert_eq!(reg.rotation(id), 0);
    }

    /// Every lookup in the game is `CATALOG.iter().find(|d| d.name == n)`, so
    /// a name used twice makes the second definition unreachable by anything
    /// that asks for it by name - monster loadouts, quest rewards, the theme
    /// table - while the shop still stocks both. The player gets two visibly
    /// different components with one name, and the theme can only translate
    /// them as one thing.
    /// Damage carries a type - physical, magic, or mind - and there is no
    /// untyped option left to fall back on: `Stats` has no bare damage field
    /// and `DamageType` has no `Untyped` variant, so an untyped number cannot
    /// be authored at all. What authoring can still produce is a blade or a
    /// spell that is simply inert, which is what this catches. Not every one
    /// of them deals damage - the Warding Sigil is a spell that shields - but
    /// every one of them has to do something.
    #[test]
    fn no_blade_or_spell_is_inert() {
        for def in CATALOG {
            if !matches!(def.kind, PieceKind::Damaging | PieceKind::Spell) {
                continue;
            }
            let does_something = def.base != Stats::ZERO
                || !def.triggers.is_empty()
                || def.adjacency.is_some()
                || def.effect.is_some()
                || def.speed_bonus != 0
                || def.power_bonus != 0;
            assert!(does_something, "{} is a {:?} piece that does nothing at all", def.name, def.kind);
        }
    }

    /// Stun and misfire used to exist only as an Oracle's class power, and
    /// the Oracle needed a crystal ball to reach - so the two most interesting
    /// curses in the game were behind a door that needed the thing behind the
    /// door to open. They are now on gear, in every slot, so a build can grow
    /// into them instead of being handed them.
    #[test]
    fn the_time_curses_are_reachable_from_every_slot() {
        use crate::curse::CurseKind;
        fn lands(t: &Trigger, want: CurseKind) -> bool {
            let is = |a: &Action| matches!(a, Action::Curse { kind, .. } if *kind == want);
            match t {
                Trigger::PerAdjacentEmpty(inner) => lands(inner, want),
                Trigger::Consume { per, .. } => is(per),
                Trigger::OnBattleStart(a) => is(a),
                Trigger::OnActivate(a)
                | Trigger::PerAdjacentItem { action: a, .. }
                | Trigger::OnAdjacentActivate(a)
                | Trigger::OnAlignedActivate(a)
                | Trigger::OnDiagonalActivate(a)
                | Trigger::OnOtherCast(a) => is(a),
                Trigger::Watch { then, .. } => is(then),
                Trigger::SpendGold { on_success, .. } => is(on_success),
                Trigger::SpendMana { on_success, on_failure, .. }
                | Trigger::Spend { on_success, on_failure, .. } => is(on_success) || is(on_failure),
            }
        }
        for want in [CurseKind::Stun, CurseKind::Misfire] {
            let carriers: Vec<&str> = CATALOG
                .iter()
                .filter(|d| d.triggers.iter().any(|t| lands(t, want)))
                .map(|d| d.name)
                .collect();
            assert!(carriers.len() >= 6, "only {} pieces land {:?}", carriers.len(), want);
            for slot in SlotKind::ALL {
                assert!(
                    CATALOG.iter().any(|d| d.fits(slot) && d.triggers.iter().any(|t| lands(t, want))),
                    "no {} lands {:?}",
                    slot.name(),
                    want
                );
            }
        }
    }

    /// A survey of pool spending found each pool could buy exactly one kind
    /// of thing: faith only ever bought defence, nature only health, rage only
    /// damage, and mana never bought growth at all. That made holding a pool a
    /// decision about *when* to spend rather than *what* for.
    #[test]
    fn every_pool_can_buy_something_outside_its_own_lane() {
        use crate::stats::StatKind;
        fn payoffs(what: Resource) -> Vec<&'static str> {
            fn walk(t: &Trigger, what: Resource, out: &mut Vec<&'static str>) {
                let tag = |a: &Action| -> &'static str {
                    match a {
                        Action::Damage { .. } => "harm",
                        Action::MindDamage { .. } => "harm",
                        Action::Curse { .. } => "harm",
                        Action::GainArmor(_) | Action::GainShield(_) => "defence",
                        Action::Grow(_) => "growth",
                        Action::GainMana(_) | Action::Gain { .. } => "pool",
                        _ => "other",
                    }
                };
                match t {
                    Trigger::PerAdjacentEmpty(inner) => walk(inner, what, out),
                    Trigger::Consume { what: w, per, .. } if *w == what => out.push(tag(per)),
                    Trigger::Spend { what: w, on_success, .. } if *w == what => {
                        out.push(tag(on_success))
                    }
                    Trigger::SpendMana { on_success, .. } if what == Resource::Mana => {
                        out.push(tag(on_success))
                    }
                    _ => {}
                }
            }
            let mut out = Vec::new();
            for d in CATALOG {
                for t in d.triggers {
                    walk(t, what, &mut out);
                }
            }
            out
        }
        for what in [Resource::Mana, Resource::Rage, Resource::Faith, Resource::Nature] {
            let p = payoffs(what);
            let mut kinds: Vec<&str> = p.clone();
            kinds.sort_unstable();
            kinds.dedup();
            assert!(
                kinds.len() >= 3,
                "{:?} can only buy {:?}; a pool with one use is a timer, not a choice",
                what,
                kinds
            );
        }
        let _ = StatKind::Health;
    }

    /// Every hold pool needs a sink somewhere other than the weapon, or a
    /// build that is not a caster banks rage, faith and nature all fight with
    /// nowhere to put any of it. Before this there was exactly one.
    #[test]
    fn the_hold_pools_can_be_spent_outside_the_weapon_slot() {
        for what in [Resource::Rage, Resource::Faith, Resource::Nature] {
            let outside: Vec<&str> = CATALOG
                .iter()
                .filter(|d| d.slot != SlotKind::Weapon)
                .filter(|d| {
                    d.triggers.iter().any(|t| {
                        matches!(t, Trigger::Spend { what: w, .. } | Trigger::Consume { what: w, .. } if *w == what)
                    })
                })
                .map(|d| d.name)
                .collect();
            assert!(
                outside.len() >= 3,
                "{:?} has only {} sink(s) off the weapon: {:?}",
                what,
                outside.len(),
                outside
            );
        }
    }

    /// A sink that empties the reserve has to pay more for a bigger reserve,
    /// or it is just a fixed threshold wearing a different name.
    #[test]
    fn emptying_a_reserve_pays_by_the_handful() {
        let consumers: Vec<&PieceDef> = CATALOG
            .iter()
            .filter(|d| d.triggers.iter().any(|t| matches!(t, Trigger::Consume { .. })))
            .collect();
        assert!(consumers.len() >= 6, "only {} pieces empty a pool", consumers.len());
        for d in &consumers {
            for t in d.triggers {
                if let Trigger::Consume { each, .. } = t {
                    assert!(*each > 0, "{} would divide by zero", d.name);
                }
            }
        }
    }

    #[test]
    fn no_two_components_share_a_name() {
        let mut seen: Vec<&str> = Vec::with_capacity(CATALOG.len());
        for def in CATALOG {
            assert!(
                !seen.contains(&def.name),
                "{} is defined twice; the second one is unreachable by name",
                def.name
            );
            seen.push(def.name);
        }
    }

    /// A piece is one thing wherever it lands. Most shapes are a single
    /// connected blob, but the Hollow Sphere is a ring of four cells touching
    /// only at the corners, and flooding the grid cell by cell used to hand
    /// the same orb back as four separate items.
    #[test]
    fn a_hollow_piece_placed_alone_is_still_one_item() {
        use crate::run::Run;
        let holey: Vec<&str> = CATALOG
            .iter()
            .filter(|d| !one_blob(d.cells))
            .map(|d| d.name)
            .collect();
        for name in holey {
            let mut run = Run::new();
            let idx = CATALOG.iter().position(|d| d.name == name).unwrap();
            let id = run.registry.alloc(idx);
            run.owned.push(id);
            let slot = CATALOG[idx].slot;
            run.equip(id, slot, 1, 1).expect("placed");
            let items = run.loadout.slot(slot).items(&run.registry);
            assert_eq!(items.len(), 1, "{} placed alone came back as {} items", name, items.len());
        }
    }

    fn one_blob(cells: &[(i8, i8)]) -> bool {
        let mut reached = vec![cells[0]];
        let mut i = 0;
        while i < reached.len() {
            let (x, y) = reached[i];
            i += 1;
            for n in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if cells.contains(&n) && !reached.contains(&n) {
                    reached.push(n);
                }
            }
        }
        reached.len() == cells.len()
    }

    #[test]
    fn no_piece_is_larger_than_a_slot() {
        for def in CATALOG {
            let s = Shape::new(def.cells);
            for turns in 0..4 {
                let r = s.rotated(turns);
                assert!(
                    r.width() <= crate::slot::SLOT_W && r.height() <= crate::slot::SLOT_H,
                    "{} does not fit a slot at rotation {}",
                    def.name,
                    turns
                );
            }
        }
    }
}

