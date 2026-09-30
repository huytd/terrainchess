//! Action selection for the AI, combining move search and spell casting decisions.

use tc_core::spell::{SpellCast, SpellId};
use tc_core::{Match, Move, Outcome};

use crate::Limits;
use crate::search::search_blocking;

const MATE: i32 = 100_000;

/// Root-level action chosen by the AI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Move(Move),
    Cast(SpellCast),
}

/// Cost tier for spells (cheapest first).
/// Common = 1, Uncommon = 2, Rare = 3.
pub fn spell_cost(id: SpellId) -> u8 {
    match id {
        SpellId::RaiseEarth
        | SpellId::LowerEarth
        | SpellId::Bridge
        | SpellId::Smite
        | SpellId::Evaporate
        | SpellId::Sprout => 1,
        SpellId::Freeze
        | SpellId::DigTunnel
        | SpellId::Shield
        | SpellId::Flood
        | SpellId::Featherfall
        | SpellId::Insight => 2,
        SpellId::Swap | SpellId::Rewind | SpellId::Curse | SpellId::Blink => 3,
    }
}

/// Chooses the best action (move or spell cast) within the time budget.
pub fn choose_action(m: &Match, limits: Limits) -> Action {
    let start = web_time::Instant::now();
    let clock = move || start.elapsed().as_secs_f64() * 1000.0;
    choose_action_with_clock(m, limits, &clock)
}

/// Chooses the best action with a custom clock function.
pub fn choose_action_with_clock(m: &Match, limits: Limits, clock: &dyn Fn() -> f64) -> Action {
    let t0 = clock();
    let cast_budget = limits.time_ms * 0.30;
    let search_time = (limits.time_ms - cast_budget).max(1.0);
    let search_limits = Limits { time_ms: search_time, ..limits };

    let (best_move, info) = search_blocking(m, search_limits, clock);
    let best_mv = best_move.or_else(|| m.legal_moves().first().copied());

    let move_score = if info.depth > 0 {
        info.score
    } else if let Some(mv) = best_mv {
        let mut next = m.clone();
        if next.play(mv).is_err() {
            next.pos.make_move(mv);
            if mv.kind == tc_core::MoveKind::Clear {
                next.terrain.get_mut(mv.to).feature = tc_core::Feature::None;
            }
        }
        let rem = (limits.time_ms - (clock() - t0)).max(10.0);
        let (_, reply) = search_blocking(&next, Limits::depth(2, rem), clock);
        -reply.score
    } else {
        -MATE
    };

    let side = m.pos.side_to_move;
    let hand = m.hand(side);
    let mut unused_spells: Vec<SpellId> = hand
        .hand
        .iter()
        .zip(hand.used.iter())
        .filter_map(|(&card, &used)| if !used { card } else { None })
        .collect();
    if unused_spells.is_empty() {
        return Action::Move(best_mv.expect("legal move"));
    }

    unused_spells.sort_by_key(|&s| spell_cost(s));
    unused_spells.dedup();

    let mut candidates = Vec::new();
    for spell in unused_spells {
        candidates.extend(m.cast_targets(spell));
    }
    candidates.sort_by_key(|cast| spell_cost(cast.spell_id()));
    candidates.truncate(24);

    if candidates.is_empty() {
        return Action::Move(best_mv.expect("legal move"));
    }

    let cast_start = clock();
    let cast_deadline = cast_start + cast_budget;
    let overall_deadline = t0 + limits.time_ms;
    let deadline = cast_deadline.min(overall_deadline);

    let mut best_cast: Option<(SpellCast, i32)> = None;

    for cast in candidates {
        if clock() >= deadline {
            break;
        }
        let remaining_time = (deadline - clock()).max(0.0);
        if remaining_time <= 0.0 {
            break;
        }

        let mut clone = m.clone();
        if clone.cast(cast).is_err() {
            continue;
        }

        let score = if cast.is_quick() {
            if let Some(outcome) = clone.outcome() {
                match outcome {
                    Outcome::Checkmate { winner } => {
                        if winner == side {
                            MATE
                        } else {
                            -MATE
                        }
                    }
                    Outcome::Draw(_) => 0,
                }
            } else {
                let (_, reply_info) = search_blocking(&clone, Limits::depth(2, remaining_time), clock);
                reply_info.score
            }
        } else if let Some(outcome) = clone.outcome() {
            match outcome {
                Outcome::Checkmate { winner } => {
                    if winner == side {
                        MATE
                    } else {
                        -MATE
                    }
                }
                Outcome::Draw(_) => 0,
            }
        } else {
            let (_, reply_info) = search_blocking(&clone, Limits::depth(2, remaining_time), clock);
            -reply_info.score
        };

        if score >= move_score + 60 && best_cast.as_ref().is_none_or(|(_, s)| score > *s) {
            best_cast = Some((cast, score));
        }
    }

    if let Some((cast, _)) = best_cast {
        Action::Cast(cast)
    } else {
        Action::Move(best_mv.expect("legal move"))
    }
}
