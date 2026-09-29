use crate::encounter::BattleResult;
use tc_core::piece::PieceKind;
use tc_core::rng::Rng;

pub fn army_value(roster: &[PieceKind]) -> u32 {
    roster
        .iter()
        .map(|&k| match k {
            PieceKind::Pawn => 1,
            PieceKind::Knight => 3,
            PieceKind::Bishop => 3,
            PieceKind::Rook => 5,
            PieceKind::Queen => 9,
            PieceKind::King => 0,
        })
        .sum()
}

/// Whether an army meets the minimum odds required for the AI to attack a camp.
pub(crate) fn can_auto_resolve_win(attacker_roster: &[PieceKind], defenders: &[PieceKind]) -> bool {
    let attacker_val = army_value(attacker_roster);
    let defender_val = army_value(defenders);
    if attacker_val < defender_val {
        return false;
    }

    let total_val = attacker_val + defender_val;
    let win_chance = if total_val == 0 { 1.0 } else { attacker_val as f32 / total_val as f32 };
    win_chance >= 0.5
}

pub fn auto_resolve(attacker_roster: &[PieceKind], defenders: &[PieceKind], rng: &mut Rng) -> BattleResult {
    let attacker_val = army_value(attacker_roster);
    let defender_val = army_value(defenders);

    // Simple resolution logic
    let total_val = attacker_val + defender_val;
    if total_val == 0 {
        return BattleResult {
            outcome: crate::encounter::Outcome::Won,
            lost: Vec::new(),

            enemy_lost: defenders.to_vec(),
        };
    }

    let win_prob = attacker_val as f32 / total_val as f32;
    let roll = rng.below(100) as f32 / 100.0;

    let won = roll < win_prob;

    // Losses scale by closeness
    let mut lost = Vec::new();
    let mut enemy_lost = Vec::new();

    if won {
        enemy_lost = defenders.to_vec();
        // If it was close, attacker loses pieces.
        let loss_prob = 1.0 - win_prob;
        for &p in attacker_roster {
            if p != PieceKind::King && rng.below(100) as f32 / 100.0 < loss_prob {
                lost.push(p);
            }
        }
    } else {
        lost = attacker_roster.iter().filter(|&&p| p != PieceKind::King).copied().collect();
        let loss_prob = win_prob;
        for &p in defenders {
            if p != PieceKind::King && rng.below(100) as f32 / 100.0 < loss_prob {
                enemy_lost.push(p);
            }
        }
    }

    BattleResult {
        outcome: if won { crate::encounter::Outcome::Won } else { crate::encounter::Outcome::Checkmated },
        lost,
        enemy_lost,
    }
}
