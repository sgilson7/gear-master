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
        shop.restock(rng, true);
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
    /// Refill the shelves.
    ///
    /// `ensure_weapon` asks for a stock that can build one from scratch. It is
    /// only true when the player has no assembled weapon, and that matters: a
    /// random six shelves almost never contains a whole recipe on its own, so
    /// forcing it every time meant two or three of the six were weapon parts
    /// for ever. Handles and blades turned up 680 times each across two
    /// hundred runs against 100 for everything else - seven times
    /// over-represented, on the one surface where a player meets the
    /// catalogue, and a standing argument for martial weapons over the other
    /// two recipes.
    pub fn restock(&mut self, rng: &mut Rng, ensure_weapon: bool) {
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
        // Enough to build *a* weapon - repaired afterwards rather than
        // reserved up front.
        //
        // Two of the six shelves used to be held back for a handle and a
        // damaging piece, every restock, for ever. There are only twenty-odd
        // of each, so across two hundred runs they turned up 680 times each
        // against 100 for everything else: seven times over-represented, on
        // the one surface where the player is supposed to meet the catalogue.
        // It also quietly argued for martial weapons by putting their parts in
        // front of you and nobody else's.
        //
        // Weapon components are two fifths of the catalogue, so a full shelf
        // can nearly always build something on its own. This only steps in
        // when it cannot, which is rare enough that the shelves stay honest.
        const RECIPES: [&[PieceKind]; 3] = [
            &[PieceKind::Handle, PieceKind::Damaging],
            &[PieceKind::Book, PieceKind::Ink, PieceKind::Spell],
            &[PieceKind::Orb, PieceKind::Spell],
        ];
        let buildable = |have: &[usize]| {
            RECIPES.iter().any(|r| {
                r.iter().all(|&k| {
                    have.iter().any(|&i| CATALOG[i].slot == SlotKind::Weapon && CATALOG[i].kind == k)
                })
            })
        };
        if ensure_weapon && !buildable(&chosen) {
            // Whichever recipe is closest to done, so the repair disturbs the
            // fewest shelves.
            let mut best: Option<(usize, &[PieceKind])> = None;
            for r in RECIPES {
                let missing = r
                    .iter()
                    .filter(|&&k| {
                        !chosen
                            .iter()
                            .any(|&i| CATALOG[i].slot == SlotKind::Weapon && CATALOG[i].kind == k)
                    })
                    .count();
                if best.as_ref().is_none_or(|(m, _)| missing < *m) {
                    best = Some((missing, r));
                }
            }
            for &k in best.expect("there are recipes").1 {
                if chosen.iter().any(|&i| CATALOG[i].slot == SlotKind::Weapon && CATALOG[i].kind == k)
                {
                    continue;
                }
                // The same exclusions as the general pool: a repair must not
                // put boss gear or a quest reward on a shelf.
                let sellable = |i: &usize| {
                    CATALOG[*i].slot == SlotKind::Weapon
                        && CATALOG[*i].kind == k
                        && !crate::piece::is_boss_only(CATALOG[*i].name)
                        && !crate::piece::is_quest_reward(CATALOG[*i].name)
                };
                let mut candidates: Vec<usize> = (0..CATALOG.len())
                    .filter(sellable)
                    .filter(|i| fresh(i) && !chosen.contains(i))
                    .collect();
                // A repeat is better than a shop you cannot build a weapon
                // from, but only once nothing fresh is left.
                if candidates.is_empty() {
                    candidates =
                        (0..CATALOG.len()).filter(sellable).filter(|i| !chosen.contains(i)).collect();
                }
                rng.shuffle(&mut candidates);
                let Some(&pick) = candidates.first() else { continue };
                // Take the shelf of something unpinned rather than growing the
                // shop past its size.
                let victim = chosen
                    .iter()
                    .position(|c| !held.contains(c) && CATALOG[*c].slot != SlotKind::Weapon);
                match victim {
                    Some(at) => chosen[at] = pick,
                    None if chosen.len() < SHOP_SIZE => chosen.push(pick),
                    None => {}
                }
            }
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


    /// Everything currently on the shelves.
    pub fn stock_defs(&self) -> Vec<&'static PieceDef> {
        self.stock.iter().map(|&i| &CATALOG[i]).collect()
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
            shop.restock(&mut rng, true);
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
        // Any of the three recipes will do. Insisting on the martial one every
        // restock is what made handles and blades seven times more common on
        // the shelves than anything else in the game.
        let mut rng = Rng::new(31);
        let mut shop = Shop::new(&mut rng);
        for round in 0..60 {
            let has = |k: PieceKind| shop.stock.iter().any(|&i| CATALOG[i].kind == k);
            let martial = has(PieceKind::Handle) && has(PieceKind::Damaging);
            let bound = has(PieceKind::Book) && has(PieceKind::Ink) && has(PieceKind::Spell);
            let ball = has(PieceKind::Orb) && has(PieceKind::Spell);
            assert!(
                martial || bound || ball,
                "round {} cannot build a weapon of any kind",
                round
            );
            shop.restock(&mut rng, true);
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
            shop.restock(&mut rng, true);
            assert_eq!(shop.stock[2], kept, "the pinned shelf should not turn over");
            assert_eq!(shop.stock.len(), SHOP_SIZE);
        }

        // And unpinning lets it go again.
        assert!(!shop.toggle_lock(2));
        let mut moved = false;
        for _ in 0..8 {
            shop.restock(&mut rng, true);
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
