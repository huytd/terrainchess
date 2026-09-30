//! The player's spell deck: which owned cards make the 15, and in what order.

use tc_core::rng::Rng;
use tc_core::spell::SpellId;

use crate::item::{Item, ItemId, ItemKind, find_item};

pub const DECK_SIZE: usize = 15;
/// Basic cards that pad a deck when the player owns fewer than 15 spell cards.
pub const FILLER: [SpellId; 5] =
    [SpellId::RaiseEarth, SpellId::LowerEarth, SpellId::Shield, SpellId::Swap, SpellId::Freeze];

/// Every card the player can put in a deck: each owned spell item's charges in owned order,
/// padded with filler up to DECK_SIZE. Never shorter than DECK_SIZE.
pub fn card_pool(owned: &[ItemId]) -> Vec<SpellId> {
    let mut pool = Vec::new();
    for id in owned {
        if let Some(Item { kind: ItemKind::Spell { spell, charges, .. }, .. }) = find_item(id) {
            for _ in 0..*charges {
                pool.push(*spell);
            }
        }
    }
    let mut filler_idx = 0;
    while pool.len() < DECK_SIZE {
        pool.push(FILLER[filler_idx % FILLER.len()]);
        filler_idx += 1;
    }
    pool
}

/// `saved` trimmed to what `pool` still holds. Cards that are no longer available leave gaps
/// that are filled in place from the rest of the pool (owned cards first, then filler), so a
/// newly won spell takes the slot a dropped filler card had. An empty `saved` gives the
/// default deck: the first DECK_SIZE cards of the pool.
pub fn reconcile(saved: &[SpellId], pool: &[SpellId]) -> Vec<SpellId> {
    if saved.is_empty() {
        return pool[..DECK_SIZE.min(pool.len())].to_vec();
    }
    let mut left = pool.to_vec();
    let mut slots: Vec<Option<SpellId>> = saved
        .iter()
        .take(DECK_SIZE)
        .map(|&c| left.iter().position(|&x| x == c).map(|i| left.remove(i)))
        .collect();
    slots.resize(DECK_SIZE, None);
    let mut rest = left.into_iter();
    slots.into_iter().filter_map(|s| s.or_else(|| rest.next())).collect()
}

/// Pool cards not in `deck` (multiset difference), in pool order.
pub fn reserve(deck: &[SpellId], pool: &[SpellId]) -> Vec<SpellId> {
    let mut used = deck.to_vec();
    pool.iter()
        .copied()
        .filter(|c| match used.iter().position(|x| x == c) {
            Some(i) => {
                used.remove(i);
                false
            }
            None => true,
        })
        .collect()
}

/// Fisher–Yates with `Rng::new(seed)`, the same shuffle as the match setup always used.
pub fn shuffle(deck: &mut [SpellId], seed: u64) {
    let mut rng = Rng::new(seed);
    for i in (1..deck.len()).rev() {
        let j = rng.below((i + 1) as u32) as usize;
        deck.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mut v: Vec<SpellId>) -> Vec<String> {
        let mut s: Vec<String> = v.drain(..).map(|c| format!("{c:?}")).collect();
        s.sort();
        s
    }

    fn spell_ids() -> Vec<ItemId> {
        crate::catalog()
            .iter()
            .filter(|i| matches!(i.kind, ItemKind::Spell { .. }))
            .map(|i| i.id.clone())
            .collect()
    }

    #[test]
    fn empty_pool_is_filler() {
        let pool = card_pool(&[]);
        assert_eq!(pool.len(), DECK_SIZE);
        for (i, c) in pool.iter().enumerate() {
            assert_eq!(*c, FILLER[i % FILLER.len()]);
        }
    }

    #[test]
    fn big_pool_has_no_filler() {
        let owned = spell_ids();
        let charges: usize = owned
            .iter()
            .map(|id| match find_item(id).unwrap().kind {
                ItemKind::Spell { charges, .. } => charges as usize,
                _ => 0,
            })
            .sum();
        let pool = card_pool(&owned);
        assert_eq!(pool.len(), charges.max(DECK_SIZE));
    }

    #[test]
    fn reconcile_empty_is_pool_prefix() {
        let pool = card_pool(&spell_ids());
        assert_eq!(reconcile(&[], &pool), pool[..DECK_SIZE].to_vec());
    }

    #[test]
    fn reconcile_keeps_order_and_fills_gaps_in_place() {
        let pool = card_pool(&[]);
        let mut saved = pool.clone();
        saved.reverse();
        assert_eq!(reconcile(&saved, &pool), saved);

        // Bridge isn't in the filler pool: slot 4 gets the first unused pool card.
        let mut saved = pool[1..].to_vec();
        saved.insert(4, SpellId::Bridge);
        let got = reconcile(&saved, &pool);
        assert_eq!(got.len(), DECK_SIZE);
        assert_eq!(got[4], pool[0]);
        assert_eq!(got[..4], saved[..4]);
        assert_eq!(got[5..], saved[5..]);
    }

    #[test]
    fn deck_plus_reserve_is_pool() {
        let pool = card_pool(&spell_ids());
        let mut saved = pool.clone();
        saved.reverse();
        let deck = reconcile(&saved, &pool);
        let mut all = deck.clone();
        all.extend(reserve(&deck, &pool));
        assert_eq!(sorted(all), sorted(pool));
    }
}
