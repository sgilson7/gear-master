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

    /// What a valid assembly in this slot needs, in one line for the UI.
    pub fn recipe_text(self) -> &'static str {
        match self {
            SlotKind::Helmet => "1 frame + 1-2 plating + up to 1 crest",
            SlotKind::Chest => "1 base + 1-3 layers",
            SlotKind::Gloves => "1 material + 1 mold",
            SlotKind::Greaves => "1 material + 1 mold",
            SlotKind::Weapon => "1 handle + 1-2 damaging + up to 2 accessories",
        }
    }
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
}

impl PieceKind {
    /// The component each recipe needs exactly one of. A core anchors an item:
    /// everything else in the slot joins the core it is nearest to, which is
    /// what lets two finished items sit flush against each other.
    pub fn is_core(self) -> bool {
        matches!(
            self,
            PieceKind::Handle | PieceKind::Frame | PieceKind::Base | PieceKind::Material
        )
    }

    pub fn name(self) -> &'static str {
        match self {
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
        }
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

/// Something an item does at the moment it activates, beyond its flat stats.
#[derive(Copy, Clone, Debug)]
pub enum Action {
    Curse { kind: CurseKind, target: Target },
    Damage { amount: i32, target: Target },
    MindDamage { amount: i32, target: Target },
    GainMana(i32),
    GainArmor(i32),
    /// Push this item's cooldown forward, so it fires sooner.
    ReduceCooldown(u32),
}

impl Action {
    pub fn describe(&self) -> String {
        match self {
            Action::Curse { kind, target } => {
                format!("apply curse of {} to {}", kind.name(), target.name())
            }
            Action::Damage { amount, target } => format!("deal {} damage to {}", amount, target.name()),
            Action::MindDamage { amount, target } => {
                format!("deal {} mind damage to {}", amount, target.name())
            }
            Action::GainMana(n) => format!("gain {} mana", n),
            Action::GainArmor(n) => format!("gain {} armor", n),
            Action::ReduceCooldown(ms) => {
                format!("cut {:.1}s off its own cooldown", *ms as f32 / 1000.0)
            }
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
}

impl Trigger {
    pub fn describe(&self) -> String {
        match self {
            Trigger::OnActivate(a) => format!("on activation, {}", a.describe()),
            Trigger::SpendMana { cost, on_success, on_failure } => format!(
                "on activation, spend {} mana: if it works, {}; if not, {}",
                cost,
                on_success.describe(),
                on_failure.describe()
            ),
            Trigger::PerAdjacentItem { action, same_slot_only } => format!(
                "on activation, per adjacent assembled {}, {}",
                if *same_slot_only { "item in this slot" } else { "item" },
                action.describe()
            ),
            Trigger::OnAdjacentActivate(a) => {
                format!("when a touching item activates, {}", a.describe())
            }
            Trigger::OnAlignedActivate(a) => format!(
                "when an item in another slot on the same rows activates, {}",
                a.describe()
            ),
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
    /// Percentage points added to the item's speed. `+100` doubles the rate,
    /// halving the cooldown. Summed across the item's pieces.
    pub speed_bonus: i32,
    /// Fires each time this piece's item activates.
    pub triggers: &'static [Trigger],
    /// What the shop charges for it.
    pub price: i32,
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
        price: 6,
    },
    PieceDef {
        name: "Iron Blade",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (0, 2), (0, 3)],
        base: Stats { damage: 8, ..Stats::new(0, 2, 0, 80) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 7,
    },
    PieceDef {
        name: "Serrated Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(1, 0), (1, 1), (0, 1), (1, 2)],
        base: Stats { damage: 6, ..Stats::new(0, 4, 0, 60) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        price: 4,
    },
    // ---- Helmet: frame, plating, crest ----
    PieceDef {
        name: "Steel Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1)],
        base: Stats { armor: 3, ..Stats::health(10) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 5,
    },
    PieceDef {
        name: "Iron Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 3, ..Stats::health(15) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 5,
    },
    PieceDef {
        name: "Visor of Focus",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0)],
        base: Stats::health(5),
        // >>> the Helmet slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Focused: +3 strength",
            stats: Stats::strength(3),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        base: Stats { armor: 5, ..Stats::health(25) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 6,
    },
    PieceDef {
        name: "Chain Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats { armor: 2, ..Stats::health(12) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 4,
    },
    PieceDef {
        name: "Plate Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats { armor: 3, ..Stats::health(18) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 5,
    },
    PieceDef {
        name: "Woven Underlayer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats::health(6),
        // >>> the Chest slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Woven: +2 regen",
            stats: Stats::regen(2),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 6,
    },
    // ---- Gloves: material + mold ----
    PieceDef {
        name: "Leather Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { armor: 1, ..Stats::strength(2) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 3,
    },
    PieceDef {
        name: "Steel Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)],
        base: Stats { armor: 2, ..Stats::new(5, 4, 0, 0) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        price: 4,
    },
    // ---- Greaves: material + mold ----
    PieceDef {
        name: "Runed Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { armor: 2, ..Stats::health(5) },
        // >>> the Greaves slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Runed: +15 health",
            stats: Stats::health(15),
        }),
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 7,
    },
    PieceDef {
        name: "Boiled Leather",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 3, ..Stats::health(10) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        price: 4,
    },
    // ---- Components with positional effects ----
    PieceDef {
        name: "Runed Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        // A cross-ish blade, so it can touch accessories on several sides.
        cells: &[(0, 0), (0, 1), (0, 2), (1, 1)],
        base: Stats { damage: 5, ..Stats::new(0, 1, 0, 45) },
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
        price: 9,
    },
    PieceDef {
        name: "Hollow Weave",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats::health(4),
        adjacency: None,
        effect: Some(Effect {
            label: "+1 strength per empty cell touching it",
            when: When::Always,
            kind: EffectKind::SelfPerEmptyCell { stat: StatKind::Strength, per: 1 },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 6,
    },
    PieceDef {
        name: "Unbound Core",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::health(8),
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
        price: 10,
    },
    PieceDef {
        name: "Cursed Blade",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (1, 1), (0, 2)],
        base: Stats::damage(10),
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        // Doubles the rate of whatever weapon it is built into.
        speed_bonus: 100,
        triggers: &[Trigger::PerAdjacentItem {
            action: Action::Curse { kind: CurseKind::Searing, target: Target::Yourself },
            same_slot_only: true,
        }],
        price: 12,
    },
    // ---- Spares, so every slot can host more than one finished item ----
    PieceDef {
        name: "Bone Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1)],
        base: Stats { armor: 2, mana: 1, ..Stats::new(6, 0, 1, 0) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 4,
    },
    PieceDef {
        name: "Hide Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 3, ..Stats::health(14) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        price: 8,
    },
    PieceDef {
        name: "Arcane Splinter",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (1, 1)],
        base: Stats { damage: 3, ..Stats::new(0, 0, 0, 20) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        // Cheap to fire, brutal when the mana is there.
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Damage { amount: 18, target: Target::Enemy },
            on_failure: Action::GainMana(1),
        }],
        price: 9,
    },
    PieceDef {
        name: "Mana Loom",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2)],
        base: Stats { mana: 6, armor: 2, ..Stats::health(18) },
        adjacency: None,
        effect: None,
        cooldown_ms: 4000,
        speed_bonus: 0,
        triggers: &[],
        price: 9,
    },
    PieceDef {
        name: "Mage's Circlet",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1)],
        base: Stats { mana: 4, ..Stats::health(8) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[],
        price: 8,
    },
    PieceDef {
        name: "Runed Lining",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats { mana: 3, ..Stats::health(6) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        price: 7,
    },
    PieceDef {
        name: "Mage's Sandals",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats { mana: 3, ..Stats::health(4) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3000,
        speed_bonus: 0,
        triggers: &[],
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
        price: 9,
    },
    PieceDef {
        name: "Hexbolt",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (0, 2)],
        base: Stats { damage: 7, mind: 2, ..Stats::new(0, 0, 0, 40) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 9,
    },
    PieceDef {
        name: "Witch's Hat",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(1, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        base: Stats { curse_resist: 15, ..Stats::health(10) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3500,
        speed_bonus: 0,
        triggers: &[Trigger::OnActivate(Action::Curse {
            kind: CurseKind::Frost,
            target: Target::Enemy,
        })],
        price: 10,
    },
    PieceDef {
        name: "Hexweave Shroud",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        base: Stats { curse_resist: 20, armor: 2, ..Stats::health(16) },
        adjacency: None,
        effect: None,
        cooldown_ms: 4500,
        speed_bonus: 0,
        triggers: &[Trigger::SpendMana {
            cost: 4,
            on_success: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
            on_failure: Action::GainArmor(4),
        }],
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
        price: 8,
    },
    PieceDef {
        name: "Witch's Stilts",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (0, 1), (0, 2), (1, 2)],
        base: Stats { curse_resist: 15, ..Stats::health(8) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3500,
        speed_bonus: 0,
        triggers: &[],
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
        triggers: &[Trigger::OnAdjacentActivate(Action::ReduceCooldown(1000))],
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
        triggers: &[Trigger::OnAdjacentActivate(Action::Damage {
            amount: 5,
            target: Target::Enemy,
        })],
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
        price: 8,
    },
    PieceDef {
        name: "Thornmail Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0)],
        base: Stats { armor: 3, ..Stats::health(8) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[Trigger::OnAdjacentActivate(Action::Damage {
            amount: 3,
            target: Target::Enemy,
        })],
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
            target: Target::Enemy,
        })],
        price: 8,
    },
    PieceDef {
        name: "Grave-Iron Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (2, 0), (2, 1)],
        base: Stats { armor: 4, ..Stats::ZERO },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        price: 8,
    },
    PieceDef {
        name: "Warding Plate",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { armor: 5, curse_resist: 10, ..Stats::health(8) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 7,
    },
    PieceDef {
        name: "Mirrored Visor",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0), (1, 1)],
        base: Stats { mind_resist: 25, ..Stats::health(6) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 8,
    },
    PieceDef {
        name: "Ironbark Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats { armor: 6, ..Stats::health(10) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        price: 11,
    },
    PieceDef {
        name: "Bonesaw",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (1, 0), (1, 1), (2, 1)],
        base: Stats { damage: 9, ..Stats::new(0, 3, 0, 30) },
        adjacency: None,
        effect: None,
        cooldown_ms: 0,
        speed_bonus: 20,
        triggers: &[],
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
        price: 6,
    },
    PieceDef {
        name: "Pathfinder Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (2, 0)],
        base: Stats { armor: 2, ..Stats::regen(2) },
        adjacency: None,
        effect: None,
        cooldown_ms: 2500,
        speed_bonus: 0,
        triggers: &[],
        price: 7,
    },
    PieceDef {
        name: "Bulwark Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats { armor: 5, ..Stats::strength(3) },
        adjacency: None,
        effect: None,
        cooldown_ms: 3500,
        speed_bonus: 0,
        triggers: &[],
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
        base: Stats::health(6),
        adjacency: None,
        effect: Some(Effect {
            label: "Unbound: +70 health and +12 armor while it stays loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat {
                stats: Stats { armor: 12, ..Stats::health(70) },
            },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        base: Stats::health(8),
        adjacency: None,
        effect: Some(Effect {
            label: "Unbound: +20 armor a tick and +8 strength while it stays loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat {
                stats: Stats { armor: 20, ..Stats::strength(8) },
            },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
            label: "Unbound: +6 regen and +30 health while it stays loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat { stats: Stats { ..Stats::new(30, 0, 6, 0) } },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
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
        base: Stats::health(5),
        adjacency: None,
        effect: Some(Effect {
            label: "Unbound: +40 health and +20% both resistances while loose",
            when: When::NotAssembled,
            kind: EffectKind::Flat {
                stats: Stats { mind_resist: 20, curse_resist: 20, ..Stats::health(40) },
            },
        }),
        cooldown_ms: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 11,
    },
];

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
