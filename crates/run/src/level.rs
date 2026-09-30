//! The endless run's stages: 10 themes, each with piece-named variants (specs/game-design.md §4).

use tc_core::PieceKind;
use tc_core::piece::Side;
use tc_core::position::Position;

/// One board setup inside a theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variant {
    pub name: &'static str,
    /// The player's army; `None` = the standard start for the theme's size.
    pub army: Option<&'static [PieceKind]>,
}

/// One of the 10 base themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub size: u8,
    /// The enemy's army when it differs from the player's (the lopsided early themes).
    pub enemy_army: Option<&'static [PieceKind]>,
    pub variants: &'static [Variant],
}

use PieceKind::*;

const fn v(name: &'static str, army: &'static [PieceKind]) -> Variant {
    Variant { name, army: Some(army) }
}

const ARMY_L5: [PieceKind; 10] = [King, Queen, Rook, Bishop, Knight, Pawn, Pawn, Pawn, Pawn, Pawn];
const ARMY_L6: [PieceKind; 8] = [King, Queen, Rook, Knight, Pawn, Pawn, Pawn, Pawn];
const ARMY_L7: [PieceKind; 12] =
    [King, Queen, Rook, Rook, Bishop, Knight, Pawn, Pawn, Pawn, Pawn, Pawn, Pawn];
const P8: [PieceKind; 8] = [Pawn; 8];
const P12: [PieceKind; 12] = [Pawn; 12];

const fn cat<const A: usize, const B: usize, const N: usize>(
    a: [PieceKind; A],
    b: [PieceKind; B],
) -> [PieceKind; N] {
    let mut out = [Pawn; N];
    let mut i = 0;
    while i < A {
        out[i] = a[i];
        i += 1;
    }
    while i < N {
        out[i] = b[i - A];
        i += 1;
    }
    out
}

const TWIN_QUEENS: [PieceKind; 16] = cat([King, Queen, Queen, Rook, Rook, Bishop, Bishop, Knight], P8);
const KNIGHT_HORDE: [PieceKind; 16] = cat([King, Queen, Rook, Rook, Knight, Knight, Knight, Knight], P8);
const BIG_BOARD: [PieceKind; 24] =
    cat([King, Queen, Queen, Rook, Rook, Rook, Bishop, Bishop, Bishop, Knight, Knight, Knight], P12);
const CAVALRY: [PieceKind; 24] =
    cat([King, Queen, Queen, Rook, Rook, Rook, Knight, Knight, Knight, Knight, Knight, Knight], P12);
const SIEGE: [PieceKind; 24] =
    cat([King, Queen, Queen, Rook, Rook, Rook, Rook, Rook, Bishop, Bishop, Knight, Knight], P12);

pub const THEMES: [Theme; 10] = [
    Theme {
        name: "Hunt",
        size: 4,
        enemy_army: Some(&[King]),
        variants: &[
            v("Queen's Hunt", &[King, Queen]),
            v("Rook's Hunt", &[King, Rook]),
            v("Bishop's Hunt", &[King, Bishop, Bishop]),
            v("Knight's Hunt", &[King, Knight, Knight, Pawn]),
        ],
    },
    Theme {
        name: "Twins",
        size: 5,
        enemy_army: Some(&[King, Pawn]),
        variants: &[
            v("Twin Rooks", &[King, Rook, Rook]),
            v("Twin Bishops", &[King, Bishop, Bishop, Pawn]),
            v("Twin Knights", &[King, Knight, Knight, Pawn]),
        ],
    },
    Theme {
        name: "Field",
        size: 6,
        enemy_army: None,
        variants: &[
            v("Knight's Field", &[King, Knight, Pawn, Pawn]),
            v("Bishop's Field", &[King, Bishop, Pawn, Pawn]),
            v("Rook's Field", &[King, Rook, Pawn, Pawn]),
        ],
    },
    Theme {
        name: "Five Pieces",
        size: 6,
        enemy_army: None,
        variants: &[
            v("Rook & Bishop", &[King, Rook, Bishop, Pawn, Pawn]),
            v("Rook & Knight", &[King, Rook, Knight, Pawn, Pawn]),
            v("Bishop & Knight", &[King, Bishop, Knight, Pawn, Pawn]),
        ],
    },
    Theme {
        name: "Gardner",
        size: 5,
        enemy_army: None,
        variants: &[
            v("Gardner Mini", &ARMY_L5),
            v("Rook Gardner", &[King, Rook, Rook, Bishop, Knight, Pawn, Pawn, Pawn, Pawn, Pawn]),
            v("Knight Gardner", &[King, Queen, Rook, Knight, Knight, Pawn, Pawn, Pawn, Pawn, Pawn]),
        ],
    },
    Theme {
        name: "Half Army",
        size: 7,
        enemy_army: None,
        variants: &[
            v("Half Army", &ARMY_L6),
            v("Half Bishops", &[King, Queen, Bishop, Bishop, Pawn, Pawn, Pawn, Pawn]),
            v("Half Knights", &[King, Queen, Knight, Knight, Pawn, Pawn, Pawn, Pawn]),
        ],
    },
    Theme {
        name: "Almost Full",
        size: 8,
        enemy_army: None,
        variants: &[
            v("Almost Full", &ARMY_L7),
            v("Bishop Pair", &[King, Queen, Rook, Rook, Bishop, Bishop, Pawn, Pawn, Pawn, Pawn, Pawn, Pawn]),
            v("Knight Pair", &[King, Queen, Rook, Rook, Knight, Knight, Pawn, Pawn, Pawn, Pawn, Pawn, Pawn]),
        ],
    },
    Theme {
        name: "Full Chess",
        size: 8,
        enemy_army: None,
        variants: &[
            Variant { name: "Full Chess", army: None },
            v("Twin Queens", &TWIN_QUEENS),
            v("Knight Horde", &KNIGHT_HORDE),
        ],
    },
    Theme {
        name: "Big Board",
        size: 12,
        enemy_army: None,
        variants: &[v("Big Board", &BIG_BOARD), v("Cavalry", &CAVALRY), v("Siege", &SIEGE)],
    },
    Theme {
        name: "Grand Battle",
        size: 16,
        enemy_army: None,
        variants: &[Variant { name: "Grand Battle", army: None }],
    },
];

/// Stages in one pass through every theme's variants.
pub const STAGES_PER_TIER: u32 = stages_per_tier();

const fn stages_per_tier() -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i < THEMES.len() {
        n += THEMES[i].variants.len() as u32;
        i += 1;
    }
    n
}

/// A stage of the endless run. `number` is 1-based and unbounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stage {
    pub number: u32,
    /// Full passes through the themes already done (0 on the first pass).
    pub tier: u32,
    pub theme: usize,
    pub variant: usize,
}

impl Stage {
    pub fn new(number: u32) -> Self {
        let number = number.max(1);
        let tier = (number - 1) / STAGES_PER_TIER;
        let mut rest = ((number - 1) % STAGES_PER_TIER) as usize;
        for (theme, t) in THEMES.iter().enumerate() {
            if rest < t.variants.len() {
                return Stage { number, tier, theme, variant: rest };
            }
            rest -= t.variants.len();
        }
        unreachable!("STAGES_PER_TIER counts every variant")
    }

    pub fn theme(&self) -> &'static Theme {
        &THEMES[self.theme]
    }

    pub fn size(&self) -> u8 {
        self.theme().size
    }

    /// "Bishop's Hunt", or "Bishop's Hunt +2" on later tiers.
    pub fn name(&self) -> String {
        let base = self.theme().variants[self.variant].name;
        if self.tier == 0 { base.to_string() } else { format!("{base} +{}", self.tier) }
    }

    /// Never decreases from one stage to the next: 0..=7 across the first tier, then +8 per
    /// tier. Used as the `floor` of `RunState::match_setup` (AI, terrain, enemy items, drafts).
    pub fn difficulty(&self) -> u8 {
        let ramp = ((self.number - 1) % STAGES_PER_TIER) * 8 / STAGES_PER_TIER;
        (ramp + 8 * self.tier).min(u8::MAX as u32) as u8
    }

    /// Search strength for `tc_ai::Limits::for_floor` (0..=7).
    pub fn ai_level(&self) -> u8 {
        self.difficulty().saturating_add(1).min(7)
    }

    /// Extra enemy piece notches (see `reinforce`): 2 per tier.
    pub fn reinforcements(&self) -> u32 {
        2 * self.tier
    }

    /// Real spells added to the enemy deck on top of one per enemy item: 2 per tier.
    pub fn extra_enemy_spells(&self) -> usize {
        2 * self.tier as usize
    }

    /// Both armies, or `None` for the standard start with no reinforcements.
    pub fn armies(&self) -> Option<[Vec<PieceKind>; 2]> {
        let theme = self.theme();
        let army = theme.variants[self.variant].army;
        let steps = self.reinforcements();
        if army.is_none() && steps == 0 {
            return None;
        }
        let player = army.map(<[_]>::to_vec).unwrap_or_else(|| start_army(theme.size));
        let mut enemy = theme.enemy_army.map(<[_]>::to_vec).unwrap_or_else(|| player.clone());
        reinforce(&mut enemy, steps, theme.size);
        Some([player, enemy])
    }

    /// Piece counts `[player, enemy]` on the starting board.
    pub fn piece_counts(&self) -> [usize; 2] {
        match self.armies() {
            Some([a, b]) => [a.len(), b.len()],
            None => {
                let n = start_army(self.size()).len();
                [n, n]
            }
        }
    }
}

/// White's pieces in `Position::start(size)`.
fn start_army(size: u8) -> Vec<PieceKind> {
    Position::start(size).pieces().filter(|(_, p)| p.side == Side::White).map(|(_, p)| p.kind).collect()
}

/// Makes a roster `steps` notches stronger. Even steps add a pawn while the home rows have room;
/// odd steps (or a full roster) promote the weakest non-king piece one rank (P, N, B, R, Q).
/// Stops when there's no room and every piece is a queen.
pub fn reinforce(roster: &mut Vec<PieceKind>, steps: u32, size: u8) {
    const LADDER: [PieceKind; 5] = [Pawn, Knight, Bishop, Rook, Queen];
    for step in 0..steps {
        let pawns = roster.iter().filter(|&&k| k == Pawn).count();
        let room = roster.len() < 2 * size as usize && pawns < size as usize;
        if step % 2 == 0 && room {
            roster.push(Pawn);
            continue;
        }
        let weakest = roster
            .iter()
            .enumerate()
            .filter_map(|(i, k)| LADDER.iter().position(|l| l == k).filter(|&r| r < 4).map(|r| (r, i)))
            .min();
        match weakest {
            Some((r, i)) => roster[i] = LADDER[r + 1],
            None if room => roster.push(Pawn),
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_one_is_queens_hunt() {
        let s = Stage::new(1);
        assert_eq!((s.theme, s.variant, s.tier), (0, 0, 0));
        assert_eq!(s.name(), "Queen's Hunt");
        assert_eq!(STAGES_PER_TIER, 29);
    }

    #[test]
    fn tier_wraps() {
        let last = Stage::new(STAGES_PER_TIER);
        assert_eq!((last.theme, last.tier), (9, 0));
        let next = Stage::new(STAGES_PER_TIER + 1);
        assert_eq!(next.tier, 1);
        assert_eq!(next.name(), "Queen's Hunt +1");
    }

    #[test]
    fn difficulty_never_drops() {
        let mut prev = 0;
        for n in 1..=200 {
            let s = Stage::new(n);
            assert!(s.difficulty() >= prev, "stage {n}");
            assert!(s.ai_level() <= 7);
            prev = s.difficulty();
        }
        assert_eq!(Stage::new(STAGES_PER_TIER).difficulty(), 7);
    }

    #[test]
    fn reinforce_adds_then_promotes() {
        let mut r = vec![King];
        reinforce(&mut r, 2, 4);
        assert_eq!(r, vec![King, Knight]);

        let mut full = start_army(8);
        reinforce(&mut full, 3, 8);
        assert_eq!(full.len(), 16);
        assert_eq!(full.iter().filter(|&&k| k == King).count(), 1);
        assert_eq!(full.iter().filter(|&&k| k == Pawn).count(), 5);

        let mut queens = vec![King, Queen, Queen, Queen, Queen, Queen, Queen, Queen];
        reinforce(&mut queens, 50, 4);
        assert_eq!(queens.len(), 8);
    }
}
