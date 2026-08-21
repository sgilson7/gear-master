//! The shop: a small rotating stock of components you spend gold on.

use crate::piece::{PieceKind, PieceDef, SlotKind, CATALOG};
use crate::rng::Rng;

/// How many components are on offer at once.
pub const SHOP_SIZE: usize = 6;
/// What you start a run with. You own nothing, so this has to cover a first
/// weapon at minimum.
pub const STARTING_GOLD: i32 = 28;
/// What a reroll costs.
pub const REROLL_COST: i32 = 1;

#[derive(Clone, Debug)]
pub struct Shop {
    /// Catalog indices currently for sale, no duplicates.
    pub stock: Vec<usize>,
    /// Shelves the player has pinned. A restock leaves these alone, so you
    /// can hold something you cannot yet afford instead of watching it go.
    pub locked: Vec<usize>,
    /// The stock before this one. Held so a restock brings genuinely new
    /// items rather than shuffling the same handful back at you.
    previous: Vec<usize>,
}

impl Shop {
    pub fn new(rng: &mut Rng) -> Self {
        let mut shop = Shop { stock: Vec::new(), locked: Vec::new(), previous: Vec::new() };
        shop.restock(rng);
        shop
    }

    /// Draw a fresh stock. Nothing repeats within it, and nothing carries over
    /// from the stock it replaces.
    ///
    /// Two shelves are reserved: every stock offers at least one weapon handle
    /// and one damaging piece. Since a run now starts owning nothing, without
    /// that guarantee an unlucky roll could leave you unable to build any
    /// weapon at all, and a player with no weapon cannot win a fight to earn
    /// the gold to reroll out of it.
    pub fn restock(&mut self, rng: &mut Rng) {
        // Whatever is pinned stays exactly where it is, and a restock fills
        // the rest of the shelves around it.
        let kept: Vec<(usize, usize)> = self
            .locked
            .iter()
            .filter_map(|&i| self.stock.get(i).map(|&def| (i, def)))
            .collect();

        let outgoing = std::mem::take(&mut self.stock);
        let held: Vec<usize> = kept.iter().map(|(_, d)| *d).collect();
        let fresh = |i: &usize| !outgoing.contains(i) && !held.contains(i);

        let mut chosen: Vec<usize> = held.clone();
        for want in [PieceKind::Handle, PieceKind::Damaging] {
            if chosen.iter().any(|&i| CATALOG[i].slot == SlotKind::Weapon && CATALOG[i].kind == want)
            {
                continue;
            }
            // The same two exclusions as the general pool. The guaranteed
            // handle and damaging piece come through here rather than there,
            // so a quest reward that happened to be a damaging piece walked
            // straight past the filter and onto a shelf.
            let sellable = |i: &usize| {
                CATALOG[*i].slot == SlotKind::Weapon
                    && CATALOG[*i].kind == want
                    && !crate::piece::is_boss_only(CATALOG[*i].name)
                    && !crate::piece::is_quest_reward(CATALOG[*i].name)
            };
            let mut candidates: Vec<usize> = (0..CATALOG.len())
                .filter(sellable)
                .filter(|i| fresh(i) && !chosen.contains(i))
                .collect();
            // If everything of this kind was on the last shelf, allow a repeat
            // rather than break the guarantee.
            if candidates.is_empty() {
                candidates = (0..CATALOG.len())
                    .filter(sellable)
                    .filter(|i| !chosen.contains(i))
                    .collect();
            }
            rng.shuffle(&mut candidates);
            if let Some(&pick) = candidates.first() {
                chosen.push(pick);
            }
        }

        let mut pool: Vec<usize> = (0..CATALOG.len())
            .filter(|i| fresh(i) && !chosen.contains(i))
            .filter(|&i| !crate::piece::is_boss_only(CATALOG[i].name))
            // A quest reward is the far side of a quest. Selling it would make
            // the quest that leads to it pointless.
            .filter(|&i| !crate::piece::is_quest_reward(CATALOG[i].name))
            .collect();
        rng.shuffle(&mut pool);
        for i in pool {
            if chosen.len() >= SHOP_SIZE {
                break;
            }
            chosen.push(i);
        }
        rng.shuffle(&mut chosen);

        // Put the pinned ones back on the shelves they were pinned to.
        for &(slot, def) in &kept {
            if let Some(at) = chosen.iter().position(|&c| c == def) {
                if slot < chosen.len() {
                    chosen.swap(at, slot);
                }
            }
        }
        self.stock = chosen;
        self.previous = outgoing;
    }

    /// Pin or unpin a shelf. Returns whether it is pinned afterwards.
    pub fn toggle_lock(&mut self, slot: usize) -> bool {
        if let Some(at) = self.locked.iter().position(|&i| i == slot) {
            self.locked.remove(at);
            false
        } else if slot < self.stock.len() {
            self.locked.push(slot);
            true
        } else {
            false
        }
    }

    pub fn is_locked(&self, slot: usize) -> bool {
        self.locked.contains(&slot)
    }


    pub fn def(&self, slot: usize) -> Option<&'static PieceDef> {
        self.stock.get(slot).map(|&i| &CATALOG[i])
    }

    pub fn price(&self, slot: usize) -> Option<i32> {
        self.def(slot).map(crate::rating::shop_price)
    }

    /// Remove the component in `slot` from the shelf, returning its catalog
    /// index. Buying it twice from one stock is not on offer.
    pub fn take(&mut self, slot: usize) -> Option<usize> {
        if slot < self.stock.len() {
            // A bought shelf is no longer pinned, and the ones after it move
            // down a place.
            self.locked.retain(|&i| i != slot);
            for i in self.locked.iter_mut() {
                if *i > slot {
                    *i -= 1;
                }
            }
            Some(self.stock.remove(slot))
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.stock.is_empty()
    }

    /// What the previous stock held — only used by the tests that check a
    /// restock really does turn the shelves over.
    pub fn previous(&self) -> &[usize] {
        &self.previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stock_has_no_duplicates() {
        let mut rng = Rng::new(1);
        let shop = Shop::new(&mut rng);
        let mut sorted = shop.stock.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), shop.stock.len());
        assert_eq!(shop.stock.len(), SHOP_SIZE);
    }

    #[test]
    fn a_restock_shares_nothing_with_the_stock_it_replaces() {
        let mut rng = Rng::new(7);
        let mut shop = Shop::new(&mut rng);
        for _ in 0..10 {
            let before = shop.stock.clone();
            shop.restock(&mut rng);
            for item in &shop.stock {
                assert!(!before.contains(item), "{:?} was on the shelf already", item);
            }
        }
    }

    #[test]
    fn buying_takes_the_item_off_the_shelf() {
        let mut rng = Rng::new(3);
        let mut shop = Shop::new(&mut rng);
        let first = shop.stock[0];
        assert_eq!(shop.take(0), Some(first));
        assert_eq!(shop.stock.len(), SHOP_SIZE - 1);
        assert!(!shop.stock.contains(&first));
        assert_eq!(shop.take(99), None, "out of range is not a purchase");
    }

    #[test]
    fn every_stock_can_build_a_weapon() {
        let mut rng = Rng::new(31);
        let mut shop = Shop::new(&mut rng);
        for round in 0..40 {
            let has_handle = shop
                .stock
                .iter()
                .any(|&i| CATALOG[i].kind == PieceKind::Handle);
            let has_damage = shop
                .stock
                .iter()
                .any(|&i| CATALOG[i].kind == PieceKind::Damaging);
            assert!(has_handle && has_damage, "round {} cannot build a weapon", round);
            shop.restock(&mut rng);
        }
    }

    #[test]
    fn a_pinned_shelf_survives_a_restock() {
        let mut rng = Rng::new(5);
        let mut shop = Shop::new(&mut rng);
        let kept = shop.stock[2];
        assert!(shop.toggle_lock(2));
        assert!(shop.is_locked(2));

        for _ in 0..8 {
            shop.restock(&mut rng);
            assert_eq!(shop.stock[2], kept, "the pinned shelf should not turn over");
            assert_eq!(shop.stock.len(), SHOP_SIZE);
        }

        // And unpinning lets it go again.
        assert!(!shop.toggle_lock(2));
        let mut moved = false;
        for _ in 0..8 {
            shop.restock(&mut rng);
            if shop.stock[2] != kept {
                moved = true;
                break;
            }
        }
        assert!(moved, "an unpinned shelf should eventually turn over");
    }

    #[test]
    fn buying_a_shelf_shifts_the_pins_after_it() {
        let mut rng = Rng::new(9);
        let mut shop = Shop::new(&mut rng);
        let pinned = shop.stock[4];
        shop.toggle_lock(4);

        shop.take(1);

        assert!(shop.is_locked(3), "the pin follows its item down a place");
        assert_eq!(shop.stock[3], pinned);
    }

    #[test]
    fn everything_on_sale_has_a_price() {
        let mut rng = Rng::new(11);
        let shop = Shop::new(&mut rng);
        for i in 0..shop.stock.len() {
            assert!(shop.price(i).unwrap() > 0);
        }
    }
}
