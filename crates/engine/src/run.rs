use crate::combat::{simulate, CombatLog, MonsterSpec, Outcome, LADDER, RUST_GOLEM};
use crate::loadout::{Loadout, SlotReport};
use crate::piece::{all_def_indices, PieceId, PieceRegistry, SlotKind, CATALOG};

/// What a run opens with: enough to assemble one item in every slot, and
/// nothing more. Everything else comes from the shop.
pub const STARTER_KIT: &[&str] = &[
    "Oak Handle",
    "Iron Blade",
    "Steel Frame",
    "Iron Plating",
    "Padded Base",
    "Chain Layer",
    "Leather Material",
    "Gripping Mold",
    "Boiled Leather",
    "Greave Mold",
];
use crate::slot::PlaceError;
use crate::rng::Rng;
use crate::shop::{Shop, STARTING_GOLD};
use crate::stats::Stats;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Phase {
    /// Arranging gear. The only phase in which the loadout can change.
    Loadout,
    /// A fight has been simulated; the GUI is replaying its log.
    Fighting,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleError {
    Place(PlaceError),
    /// Tried to change the loadout mid-fight.
    LoadoutLocked,
    NotEquipped,
    /// Tried to buy something you can't afford.
    NotEnoughGold { need: i32, have: i32 },
    /// Tried to buy from an empty shelf.
    NothingThere,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleError::Place(e) => write!(f, "{}", e),
            RuleError::LoadoutLocked => write!(f, "can't change gear during a fight"),
            RuleError::NotEquipped => write!(f, "that piece isn't equipped"),
            RuleError::NotEnoughGold { need, have } => {
                write!(f, "costs {} gold, you have {}", need, have)
            }
            RuleError::NothingThere => write!(f, "nothing for sale there"),
        }
    }
}

impl From<PlaceError> for RuleError {
    fn from(e: PlaceError) -> Self {
        RuleError::Place(e)
    }
}

pub struct Run {
    pub registry: PieceRegistry,
    /// Every component the player owns, in a stable display order. What is in
    /// the inventory is derived from this minus what is in the slots, so the
    /// two can never disagree.
    pub owned: Vec<PieceId>,
    pub loadout: Loadout,
    pub phase: Phase,
    /// Set by `begin_fight`, cleared by `back_to_loadout`.
    pub log: Option<CombatLog>,
    pub gold: i32,
    pub shop: Shop,
    /// How far up the monster ladder you are.
    pub rung: usize,
    pub wins: u32,
    pub losses: u32,
    /// Set once a fight's result has been banked, so the reward can't be
    /// claimed twice by replaying the same log.
    settled: bool,
    rng: Rng,
}

impl Default for Run {
    fn default() -> Self {
        Self::new()
    }
}

impl Run {
    /// A fresh run: a small starter kit, 20 gold and a stocked shop.
    pub fn new() -> Self {
        Self::seeded(0x5EED_1234_ABCD_0001)
    }

    /// Same, with the shop's rolls pinned so a test can predict them.
    pub fn seeded(seed: u64) -> Self {
        let mut registry = PieceRegistry::new();
        let mut owned = Vec::new();
        for name in STARTER_KIT {
            if let Some(d) = CATALOG.iter().position(|p| &p.name == name) {
                owned.push(registry.alloc(d));
            }
        }
        let mut rng = Rng::new(seed);
        let shop = Shop::new(&mut rng);
        Run {
            registry,
            owned,
            loadout: Loadout::new(),
            phase: Phase::Loadout,
            log: None,
            gold: STARTING_GOLD,
            shop,
            rung: 0,
            wins: 0,
            losses: 0,
            settled: false,
            rng,
        }
    }

    /// Every component in the catalog, for the preset, the tests, and the
    /// AUTO-BUILD button. Bypasses the shop entirely.
    pub fn with_all_pieces() -> Self {
        let mut run = Self::new();
        run.owned.clear();
        run.registry = PieceRegistry::new();
        run.owned = all_def_indices().into_iter().map(|d| run.registry.alloc(d)).collect();
        run
    }

    /// The monster you are facing now.
    pub fn monster(&self) -> &'static MonsterSpec {
        &LADDER[self.rung.min(LADDER.len() - 1)]
    }

    /// True once the ladder has been cleared.
    pub fn ladder_complete(&self) -> bool {
        self.rung >= LADDER.len()
    }

    /// Buy the component on shelf `slot`.
    pub fn buy(&mut self, slot: usize) -> Result<PieceId, RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let price = self.shop.price(slot).ok_or(RuleError::NothingThere)?;
        if self.gold < price {
            return Err(RuleError::NotEnoughGold { need: price, have: self.gold });
        }
        let def = self.shop.take(slot).ok_or(RuleError::NothingThere)?;
        self.gold -= price;
        let id = self.registry.alloc(def);
        self.owned.push(id);
        Ok(id)
    }

    /// Sell a component back for half its price, rounded down. Equipped pieces
    /// come off first.
    pub fn sell(&mut self, id: PieceId) -> Result<i32, RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let refund = self.registry.def(id).price / 2;
        self.loadout.remove_anywhere(id);
        self.owned.retain(|&o| o != id);
        self.gold += refund;
        Ok(refund)
    }

    /// Bank the result of the fight just watched: pay the bounty, advance the
    /// ladder, and turn the shop over. Idempotent, so the GUI can call it when
    /// playback finishes without worrying about repeats.
    pub fn settle(&mut self) -> Option<i32> {
        if self.settled {
            return None;
        }
        let outcome = self.log.as_ref()?.outcome;
        self.settled = true;
        let reward = match outcome {
            Outcome::Victory => {
                let bounty = self.monster().bounty;
                self.gold += bounty;
                self.wins += 1;
                self.rung += 1;
                Some(bounty)
            }
            _ => {
                self.losses += 1;
                None
            }
        };
        // New shelves after every battle, win or lose.
        self.shop.restock(&mut self.rng);
        reward
    }

    /// Components not currently in a slot, in stable order.
    pub fn inventory(&self) -> Vec<PieceId> {
        self.owned
            .iter()
            .copied()
            .filter(|id| self.loadout.slot_holding(*id).is_none())
            .collect()
    }

    pub fn is_equipped(&self, id: PieceId) -> bool {
        self.loadout.slot_holding(id).is_some()
    }

    /// Can `id` be dropped into `kind` with its anchor at `(ax, ay)`? Pure
    /// query — the GUI calls this every frame while dragging so it can tint
    /// the preview, and must never work the answer out for itself.
    pub fn can_equip(
        &self,
        id: PieceId,
        kind: SlotKind,
        ax: u8,
        ay: u8,
    ) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        // A piece being moved within its own slot shouldn't collide with
        // itself; `Slot::can_place` already allows that. Moving between slots
        // is checked against the destination as it currently stands, which is
        // correct because the source slot is a different grid.
        Ok(self.loadout.can_place(&self.registry, id, kind, ax, ay)?)
    }

    /// Place `id` into `kind` at `(ax, ay)`, taking it out of wherever it was.
    /// Ordering:
    ///   1. reject if the loadout is locked or the destination doesn't fit
    ///   2. lift the piece out of any slot currently holding it
    ///   3. write it into the destination
    pub fn equip(&mut self, id: PieceId, kind: SlotKind, ax: u8, ay: u8) -> Result<(), RuleError> {
        self.can_equip(id, kind, ax, ay)?;
        self.loadout.remove_anywhere(id);
        self.loadout.slot_mut(kind).place(&self.registry, id, ax, ay);
        Ok(())
    }

    /// Take `id` off and return it to the inventory.
    pub fn unequip(&mut self, id: PieceId) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        if !self.is_equipped(id) {
            return Err(RuleError::NotEquipped);
        }
        self.loadout.remove_anywhere(id);
        Ok(())
    }

    /// Rotate `id` a quarter turn clockwise. A piece already in a slot only
    /// turns if it still fits afterwards — otherwise the rotation is undone,
    /// so a rejected rotation leaves the world untouched.
    pub fn rotate(&mut self, id: PieceId) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let before = self.registry.rotation(id);
        self.registry.rotate_cw(id);

        if let Some(kind) = self.loadout.slot_holding(id) {
            let anchor = self
                .loadout
                .slot(kind)
                .anchor_of(id)
                .expect("a held piece has an anchor");
            // Re-place from scratch: clear the old footprint, then test.
            self.loadout.remove_anywhere(id);
            match self.loadout.can_place(&self.registry, id, kind, anchor.0, anchor.1) {
                Ok(()) => {
                    self.loadout.slot_mut(kind).place(&self.registry, id, anchor.0, anchor.1);
                }
                Err(e) => {
                    self.registry.set_rotation(id, before);
                    self.loadout.slot_mut(kind).place(&self.registry, id, anchor.0, anchor.1);
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    /// A complete, legal loadout. Used by the "auto-build" button and by the
    /// tests, so the two can never drift apart.
    ///
    /// Deliberately shows off the mechanics rather than maxing the numbers:
    /// chest, gloves and greaves each carry **two** separate finished items,
    /// the weapon's Runed Edge doubles the Ruby Inlay next to it, and the
    /// Hollow Weave sits out in open space where its empty-cell bonus counts.
    /// Fields are `(name, slot, anchor x, anchor y, quarter turns)`.
    pub fn apply_preset(&mut self) {
        for kind in SlotKind::ALL {
            self.loadout.slot_mut(kind).clear();
        }
        const PRESET: &[(&str, SlotKind, u8, u8, u8)] = &[
            // Helmet — one item: frame + two plating (one is the bonus piece)
            // + crest.
            ("Steel Frame", SlotKind::Helmet, 0, 0, 0),
            ("Iron Plating", SlotKind::Helmet, 0, 2, 0),
            ("Visor of Focus", SlotKind::Helmet, 0, 4, 0),
            ("Crest of Vigor", SlotKind::Helmet, 3, 0, 0),
            // Chest — two items. The first fills the top-left; the second
            // hangs off the right-hand column with a gap between them, so the
            // Hollow Weave keeps five empty cells against its flank.
            ("Padded Base", SlotKind::Chest, 0, 0, 0),
            ("Chain Layer", SlotKind::Chest, 0, 3, 0),
            ("Woven Underlayer", SlotKind::Chest, 0, 4, 0),
            ("Hollow Weave", SlotKind::Chest, 5, 2, 1),
            ("Hide Base", SlotKind::Chest, 3, 6, 0),
            // Gloves — two items.
            ("Leather Material", SlotKind::Gloves, 0, 0, 0),
            ("Gripping Mold", SlotKind::Gloves, 2, 0, 0),
            ("Steel Material", SlotKind::Gloves, 0, 4, 0),
            ("Gauntlet Mold", SlotKind::Gloves, 2, 4, 0),
            // Greaves — two items.
            ("Runed Material", SlotKind::Greaves, 0, 0, 0),
            ("Greave Mold", SlotKind::Greaves, 2, 0, 0),
            ("Boiled Leather", SlotKind::Greaves, 0, 4, 0),
            ("Runner's Mold", SlotKind::Greaves, 3, 4, 0),
            // Weapon — one item, built around the Runed Edge so both
            // accessories sit against it.
            ("Balanced Grip", SlotKind::Weapon, 0, 0, 0),
            ("Runed Edge", SlotKind::Weapon, 1, 0, 0),
            ("Ruby Inlay", SlotKind::Weapon, 2, 0, 0),
            ("Balance Weight", SlotKind::Weapon, 2, 2, 0),
        ];
        // The preset names specific components, so grant any the player has
        // not bought. It is a demo button, not a way to dodge the shop.
        for &(name, ..) in PRESET {
            if self.find_by_name(name).is_none() {
                if let Some(d) = CATALOG.iter().position(|p| p.name == name) {
                    let id = self.registry.alloc(d);
                    self.owned.push(id);
                }
            }
        }
        for &(name, kind, ax, ay, rot) in PRESET {
            let Some(id) = self.find_by_name(name) else { continue };
            self.registry.set_rotation(id, rot);
            self.loadout.remove_anywhere(id);
            if self.loadout.can_place(&self.registry, id, kind, ax, ay).is_ok() {
                self.loadout.slot_mut(kind).place(&self.registry, id, ax, ay);
            }
        }
    }

    /// First owned component with this catalog name.
    pub fn find_by_name(&self, name: &str) -> Option<PieceId> {
        self.owned
            .iter()
            .copied()
            .find(|&id| self.registry.def(id).name == name)
    }

    /// Strip every slot and reset rotations.
    pub fn clear_all(&mut self) {
        for kind in SlotKind::ALL {
            self.loadout.slot_mut(kind).clear();
        }
        for &id in &self.owned {
            self.registry.set_rotation(id, 0);
        }
    }

    pub fn clear_slot(&mut self, kind: SlotKind) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        self.loadout.slot_mut(kind).clear();
        Ok(())
    }

    pub fn reports(&self) -> Vec<SlotReport> {
        self.loadout.reports(&self.registry)
    }

    pub fn report(&self, kind: SlotKind) -> SlotReport {
        self.loadout.report(&self.registry, kind)
    }

    /// Base character stats plus every slot's contribution.
    pub fn player_stats(&self) -> Stats {
        self.loadout.total_stats(&self.registry)
    }

    /// Activation profiles for every assembled item — what combat runs on.
    pub fn combat_items(&self) -> Vec<crate::loadout::ItemProfile> {
        self.loadout.combat_items(&self.registry)
    }

    /// Simulate the whole fight against `spec` and enter the replay phase.
    pub fn fight(&mut self, spec: &MonsterSpec) -> &CombatLog {
        let log = simulate(self.player_stats(), &self.combat_items(), spec);
        self.phase = Phase::Fighting;
        self.settled = false;
        self.log = Some(log);
        self.log.as_ref().expect("just set")
    }

    /// Fight whatever is next on the ladder.
    pub fn fight_next(&mut self) -> &CombatLog {
        let spec = *self.monster();
        self.fight(&spec)
    }

    /// Simulate against the original opponent, ladder position ignored.
    pub fn begin_fight(&mut self) -> &CombatLog {
        self.fight(&RUST_GOLEM)
    }

    /// Return to gear-arranging and discard the fight.
    pub fn back_to_loadout(&mut self) {
        self.phase = Phase::Loadout;
        self.log = None;
    }
}
