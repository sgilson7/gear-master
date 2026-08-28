//! The shop: a small rotating stock of components you spend gold on.

use crate::piece::{PieceKind, PieceDef, SlotKind, CATALOG};
use crate::rng::Rng;

/// How many components are on offer at once.
///
/// **Seven since 2026-08-27.** It was six for the whole of the game's life,
/// and six is a shelf that shows one slot's worth of choice at a time: the
/// tilt deals the five slots round-robin, so at six a run sees one or two
/// pieces of some slots and none of others, and the slot it saw nothing of is
/// the slot it does not build. A seventh is a whole extra draw against the
/// same pool and it fits the strip the interface already draws - a card is 126
/// wide with a 10 gap and the band is 1,186, so seven need 1,088 and eight
/// would not fit.
///
/// It is a shelf count and not a curated one: `stock_exactly` ignores it, so
/// the pub's six shelves and a town's curated five are unmoved.
pub const SHOP_SIZE: usize = 7;
/// How hard the shelf tilts toward the slots with the most pieces.
///
/// 1.0 deals every slot in proportion to its catalogue - perfectly even
/// components, and the weapon on 54.8% of every shelf because it is two fifths
/// of the pieces. 0.5 deals the five slots nearly evenly - a fair shelf, and a
/// chest piece 2.5x as likely as a weapon piece, which
/// `avail::the_shelves_are_not_the_same_few_things_every_time` refuses at 3.7x.
///
/// The two rules pull against each other and this is where both hold.
const SHELF_TILT: f32 = 0.9;

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
    /// Whether the mind lane's pool has been earned yet.
    ///
    /// Shut until THE THRESHOLD is cleared, and while it is shut nothing that
    /// banks Insight or stacks Dread is dealt. A gate on the shelf rather than
    /// on the piece, because what is for sale is a property of the run and not
    /// of the catalogue - and because the run is the only thing that knows.
    pub insight_open: bool,
    /// Kinds a standing order says every shelf must offer.
    ///
    /// Repaired after the shelves are dealt rather than reserved before them,
    /// the same way the weapon guarantee is and for the same reason: holding
    /// shelves for a kind, every restock, for ever, is what made handles and
    /// blades seven times over-represented the last time anybody tried it.
    pub guaranteed: Vec<PieceKind>,
    /// Whether the first reroll after a restock costs nothing.
    pub free_first_reroll: bool,
}

impl Shop {
    /// Put exactly these on the shelves and nothing else.
    ///
    /// For an offer that is not a shop's own choosing - somebody has laid five
    /// things out for you, and a reroll would be missing the point. Pins are
    /// cleared with everything else: what was on the shelf is gone.
    pub fn stock_exactly(&mut self, names: &[&str]) {
        self.locked.clear();
        self.previous = std::mem::take(&mut self.stock);
        self.stock = names
            .iter()
            .filter_map(|n| CATALOG.iter().position(|d| d.name == *n))
            .collect();
    }

    pub fn new(rng: &mut Rng) -> Self {
        let mut shop =
            Shop {
                stock: Vec::new(),
                locked: Vec::new(),
                previous: Vec::new(),
                insight_open: false,
                guaranteed: Vec::new(),
                free_first_reroll: false,
            };
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
            .filter(|&i| !crate::piece::is_event_only(CATALOG[i].name))
            // Town gear is bought in a town. The five curated shelves are the
            // reason to walk into a settlement rather than past it, and an
            // underlay is ground rather than kit - neither belongs on a stall
            // by the side of the road.
            .filter(|&i| !crate::piece::is_off_the_road(&CATALOG[i]))
            // A pool nobody has been given yet is a piece that does nothing.
            .filter(|&i| self.insight_open || !crate::piece::touches_insight(&CATALOG[i]))
            .collect();
        // Dealt a slot at a time, not drawn uniformly from the catalogue.
        //
        // A uniform pool is a pool the weapon owns, because the weapon is two
        // fifths of the catalogue: measured over 400 seeded runs and six
        // restocks each, the weapon took **54.8%** of every shelf against a
        // 36.7% share of the pieces, and the four armour slots got ten to
        // thirteen per cent apiece. The shop is the one surface where a player
        // meets the catalogue, and on it the game was not five slots, it was a
        // weapon and some accessories.
        //
        // So the shelves are dealt round-robin over a shuffled slot order:
        // whatever a slot has in the catalogue, it gets its turn. That is not
        // the reservation that was tried and reverted - that one held shelves
        // for a *kind*, which made handles and blades seven times
        // over-represented and quietly argued for martial weapons over the
        // other two recipes. This holds nothing for anything; it just stops
        // one slot taking the whole shelf by weight of numbers.
        rng.shuffle(&mut pool);
        // One ticket per square root of a slot's catalogue, which is the
        // compromise the two evenness rules leave room for.
        //
        // Dealing the five slots in equal turns fixes the shelf mix and breaks
        // something else: a slot's share is then spread over however many
        // pieces it has, so a chest piece (69 of them) turns up two and a half
        // times as often as a weapon piece (172), and
        // `avail::the_shelves_are_not_the_same_few_things_every_time` says 3.7x
        // and refuses. Dealing in proportion to the catalogue is what we had,
        // and that is the weapon taking 55% of every shelf.
        //
        // So the exponent is the dial and it sits between them. At 1.0 a slot
        // is dealt in proportion to its catalogue: every component is exactly
        // as likely as every other and the weapon takes the shelf. At 0.5 the
        // shelf is nearly even between slots and a chest piece is two and a
        // half times as likely as a weapon piece. `SHELF_TILT` is set where
        // both tests pass, which is the only place either of them is happy.
        let mut tickets: Vec<SlotKind> = Vec::new();
        for k in SlotKind::ALL {
            // Counted over the **pool**, not over `CATALOG`.
            //
            // It was the catalogue, which meant every piece the filters above
            // had just removed still bought its slot a ticket: boss gear,
            // quest rewards, town stock, the mind lane before it is open, and
            // a hundred and twenty event-only components. A slot was dealt in
            // proportion to how much of it exists rather than to how much of
            // it is for sale, and the two have not been the same number since
            // the Unwinding appended thirty-one rewards.
            //
            // Found by appending eight unsellable components and watching
            // `avail::the_shelves_are_not_the_same_few_things_every_time`
            // move - the shelves shifted because the catalogue grew in places
            // no shelf can reach.
            let n = pool.iter().filter(|&&i| CATALOG[i].slot == k).count();
            let weight = ((n as f32).powf(SHELF_TILT) / 6.0).round().max(1.0) as usize;
            for _ in 0..weight {
                tickets.push(k);
            }
        }
        rng.shuffle(&mut tickets);
        let mut round = 0usize;
        while chosen.len() < SHOP_SIZE {
            let before = chosen.len();
            for &want in &tickets {
                if chosen.len() >= SHOP_SIZE {
                    break;
                }
                if let Some(pos) = pool
                    .iter()
                    .position(|&i| CATALOG[i].slot == want && !chosen.contains(&i))
                {
                    chosen.push(pool.remove(pos));
                }
            }
            // Nothing left in any slot: fall back to whatever the pool still
            // holds, so a heavily-filtered catalogue still fills the shelves.
            if chosen.len() == before {
                match pool.iter().position(|i| !chosen.contains(i)) {
                    Some(pos) => chosen.push(pool.remove(pos)),
                    None => break,
                }
            }
            round += 1;
            if round > SHOP_SIZE + 5 {
                break;
            }
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
                // put boss gear, a quest reward or a town's stock on a shelf.
                // Kettleworks Pin is a damaging piece, so leaving town gear
                // out of the pool and not out of the repair meant the road
                // still handed it over whenever somebody turned up unarmed.
                let insight_open = self.insight_open;
                let sellable = move |i: &usize| {
                    CATALOG[*i].slot == SlotKind::Weapon
                        && CATALOG[*i].kind == k
                        && !crate::piece::is_boss_only(CATALOG[*i].name)
                        && !crate::piece::is_quest_reward(CATALOG[*i].name)
                        && !crate::piece::is_event_only(CATALOG[*i].name)
                        && !crate::piece::is_off_the_road(&CATALOG[*i])
                        && (insight_open || !crate::piece::touches_insight(&CATALOG[*i]))
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

        // A standing order, honoured after everything else has had its turn.
        for want in self.guaranteed.clone() {
            if chosen.iter().any(|&c| CATALOG[c].kind == want) {
                continue;
            }
            let mut candidates: Vec<usize> = (0..CATALOG.len())
                .filter(|&i| CATALOG[i].kind == want && !chosen.contains(&i))
                .filter(|&i| !crate::piece::is_boss_only(CATALOG[i].name))
                .filter(|&i| !crate::piece::is_quest_reward(CATALOG[i].name))
                .filter(|&i| !crate::piece::is_event_only(CATALOG[i].name))
                .filter(|&i| !crate::piece::is_off_the_road(&CATALOG[i]))
                .filter(|&i| self.insight_open || !crate::piece::touches_insight(&CATALOG[i]))
                .collect();
            rng.shuffle(&mut candidates);
            let Some(&pick) = candidates.first() else { continue };
            match chosen.iter().position(|c| !held.contains(c)) {
                Some(at) => chosen[at] = pick,
                None if chosen.len() < SHOP_SIZE => chosen.push(pick),
                None => {}
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

    /// Put one particular thing on a shelf, whatever else was going to be
    /// there.
    ///
    /// For things that are coming *back* rather than being offered - a piece
    /// left on consignment - so the shelf it lands on is whichever one nobody
    /// has pinned.
    pub fn put_on_a_shelf(&mut self, def: usize) {
        if self.stock.contains(&def) {
            return;
        }
        match (0..self.stock.len()).find(|i| !self.locked.contains(i)) {
            Some(at) => self.stock[at] = def,
            None => self.stock.push(def),
        }
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
