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
    },
    PieceDef {
        name: "Iron Blade",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(0, 0), (0, 1), (0, 2), (0, 3)],
        base: Stats::new(0, 2, 0, 80),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Serrated Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        cells: &[(1, 0), (1, 1), (0, 1), (1, 2)],
        base: Stats::new(0, 4, 0, 60),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Ruby Inlay",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0)],
        base: Stats::strength(3),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Balance Weight",
        slot: SlotKind::Weapon,
        kind: PieceKind::Accessory,
        cells: &[(0, 0), (1, 0)],
        base: Stats::power(25),
        adjacency: None,
        effect: None,
    },
    // ---- Helmet: frame, plating, crest ----
    PieceDef {
        name: "Steel Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1)],
        base: Stats::health(10),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Iron Plating",
        slot: SlotKind::Helmet,
        kind: PieceKind::Plating,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats::health(15),
        adjacency: None,
        effect: None,
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
    },
    PieceDef {
        name: "Crest of Vigor",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0), (0, 1)],
        base: Stats::regen(1),
        adjacency: None,
        effect: None,
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
        base: Stats::health(25),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Chain Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats::health(12),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Plate Layer",
        slot: SlotKind::Chest,
        kind: PieceKind::Layer,
        cells: &[(0, 0), (1, 0), (2, 0), (3, 0)],
        base: Stats::health(18),
        adjacency: None,
        effect: None,
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
    },
    // ---- Gloves: material + mold ----
    PieceDef {
        name: "Leather Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::strength(2),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Steel Material",
        slot: SlotKind::Gloves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)],
        base: Stats::new(5, 4, 0, 0),
        adjacency: None,
        effect: None,
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
    },
    PieceDef {
        name: "Gripping Mold",
        slot: SlotKind::Gloves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1)],
        base: Stats::power(15),
        adjacency: None,
        effect: None,
    },
    // ---- Greaves: material + mold ----
    PieceDef {
        name: "Runed Material",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::health(5),
        // >>> the Greaves slot's adjacency bonus <<<
        adjacency: Some(Adjacency {
            label: "Runed: +15 health",
            stats: Stats::health(15),
        }),
        effect: None,
    },
    PieceDef {
        name: "Boiled Leather",
        slot: SlotKind::Greaves,
        kind: PieceKind::Material,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats::health(10),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Greave Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (1, 1), (1, 2)],
        base: Stats::regen(1),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Runner's Mold",
        slot: SlotKind::Greaves,
        kind: PieceKind::Mold,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::regen(2),
        adjacency: None,
        effect: None,
    },
    // ---- Components with positional effects ----
    PieceDef {
        name: "Runed Edge",
        slot: SlotKind::Weapon,
        kind: PieceKind::Damaging,
        // A cross-ish blade, so it can touch accessories on several sides.
        cells: &[(0, 0), (0, 1), (0, 2), (1, 1)],
        base: Stats::new(0, 1, 0, 45),
        adjacency: None,
        effect: Some(Effect {
            label: "adjacent accessories give double strength",
            when: When::Assembled,
            kind: EffectKind::DoubleNeighbor {
                kind: PieceKind::Accessory,
                stat: StatKind::Strength,
            },
        }),
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
    },
    // ---- Spares, so every slot can host more than one finished item ----
    PieceDef {
        name: "Bone Frame",
        slot: SlotKind::Helmet,
        kind: PieceKind::Frame,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1)],
        base: Stats::new(6, 0, 1, 0),
        adjacency: None,
        effect: None,
    },
    PieceDef {
        name: "Hide Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Base,
        cells: &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
        base: Stats::health(14),
        adjacency: None,
        effect: None,
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
    fn every_slot_has_exactly_one_adjacency_bonus() {
        for slot in SlotKind::ALL {
            let n = CATALOG
                .iter()
                .filter(|d| d.slot == slot && d.adjacency.is_some())
                .count();
            assert_eq!(n, 1, "{} should have exactly one bonus piece", slot.name());
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
