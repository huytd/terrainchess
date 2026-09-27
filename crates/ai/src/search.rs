//! Resumable iterative-deepening search.
//!
//! [`SearchJob::step`] searches root moves one at a time until its time slice runs
//! out, then returns so the caller can render a frame. The overall deadline is also
//! checked inside the tree, so a deep subtree can't overrun it.

use tc_core::movegen::Ctx;
use tc_core::rng::Rng;
use tc_core::{Match, Move, MoveKind, Position, Rules, Terrain};

use crate::Limits;
use crate::eval::{evaluate, piece_value};
use crate::tt::{Bound, Entry, Tt};

const INF: i32 = 1_000_000;
const MATE: i32 = 100_000;
const MAX_PLY: usize = 64;
/// Check the clock every this many nodes.
const CLOCK_EVERY: u64 = 512;

fn is_mate(score: i32) -> bool {
    score.abs() > MATE - MAX_PLY as i32 * 2
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SearchInfo {
    /// Deepest fully completed iteration.
    pub depth: u8,
    pub nodes: u64,
    /// Score of the best move for the side to move, in centipawns.
    pub score: i32,
    pub best: Option<Move>,
}

pub struct SearchJob {
    terrain: Terrain,
    rules: Rules,
    root: Position,
    /// Positions before the root, for repetition.
    history: Vec<u64>,
    limits: Limits,
    tt: Tt,
    killers: [[Option<Move>; 2]; MAX_PLY],
    path: Vec<u64>,
    nodes: u64,
    start_ms: Option<f64>,
    deadline_ms: f64,
    aborted: bool,

    // Iteration state, kept between slices.
    root_moves: Vec<(Move, i32)>,
    depth: u8,
    next_root: usize,
    alpha: i32,
    iter_best: Option<(Move, i32)>,
    info: SearchInfo,
    result: Option<Move>,
}

/// Run a job to completion; for tests, tools and native self-play.
pub fn search_blocking(game: &Match, limits: Limits, clock: &dyn Fn() -> f64) -> (Option<Move>, SearchInfo) {
    let mut job = SearchJob::new(game, limits);
    loop {
        if let Some(mv) = job.step(clock, f64::INFINITY) {
            return (Some(mv), job.info());
        }
        if job.is_finished() {
            return (None, job.info());
        }
    }
}

impl SearchJob {
    pub fn new(game: &Match, limits: Limits) -> Self {
        let ctx = game.ctx();
        let root_moves = ctx.legal_moves(&game.pos).into_iter().map(|m| (m, 0)).collect();
        let history = game.history();
        SearchJob {
            terrain: game.terrain.clone(),
            rules: game.rules.clone(),
            root: game.pos.clone(),
            history: history[..history.len().saturating_sub(1)].to_vec(),
            limits,
            tt: Tt::new(18),
            killers: [[None; 2]; MAX_PLY],
            path: Vec::with_capacity(MAX_PLY),
            nodes: 0,
            start_ms: None,
            deadline_ms: f64::INFINITY,
            aborted: false,
            root_moves,
            depth: 1,
            next_root: 0,
            alpha: -INF,
            iter_best: None,
            info: SearchInfo::default(),
            result: None,
        }
    }

    pub fn info(&self) -> SearchInfo {
        SearchInfo { nodes: self.nodes, ..self.info }
    }

    /// No legal moves at the root (the game is already over).
    pub fn is_finished(&self) -> bool {
        self.root_moves.is_empty() || self.result.is_some()
    }

    /// Search for up to `slice_ms`. Returns the chosen move once the search is done.
    pub fn step(&mut self, clock: &dyn Fn() -> f64, slice_ms: f64) -> Option<Move> {
        if let Some(mv) = self.result {
            return Some(mv);
        }
        if self.root_moves.is_empty() {
            return None;
        }
        let now = clock();
        let start = *self.start_ms.get_or_insert(now);
        self.deadline_ms = start + self.limits.time_ms;
        let slice_end = now + slice_ms;
        if self.root_moves.len() == 1 {
            return self.finish(Some(self.root_moves[0].0));
        }

        while clock() < slice_end {
            if self.next_root == 0 {
                // Starting a new iteration: stop if it probably can't finish in time.
                let elapsed = clock() - start;
                let done = self.info.depth >= self.limits.max_depth
                    || (self.info.depth > 0 && elapsed > self.limits.time_ms * 0.45)
                    || is_mate(self.info.score) && self.info.depth > 0;
                if done {
                    return self.finish(self.info.best);
                }
                self.alpha = -INF;
                self.iter_best = None;
            }

            let (mv, _) = self.root_moves[self.next_root];
            let score = self.search_root_move(mv, clock);
            if self.aborted {
                // Out of time mid-iteration. The previous best is searched first, so a
                // partial result that beat it is still trustworthy.
                let partial = self.iter_best.filter(|_| self.next_root > 0).map(|b| b.0);
                return self.finish(partial.or(self.info.best));
            }
            self.root_moves[self.next_root].1 = score;
            if score > self.alpha {
                self.alpha = score;
                self.iter_best = Some((mv, score));
            }
            self.next_root += 1;

            if self.next_root == self.root_moves.len() {
                let (best, score) = self.iter_best.expect("root has moves");
                self.info = SearchInfo { depth: self.depth, nodes: self.nodes, score, best: Some(best) };
                // Best first next time; the others keep their (bound) scores for ordering.
                self.root_moves.sort_by_key(|&(m, s)| if m == best { -INF - 1 } else { -s });
                self.depth += 1;
                self.next_root = 0;
            }
        }
        None
    }

    fn finish(&mut self, best: Option<Move>) -> Option<Move> {
        let mut choice = best.or(self.root_moves.first().map(|m| m.0));
        // Early floors: sometimes take another of the top few moves instead.
        let mut rng = Rng::new(self.limits.seed ^ self.root.hash());
        if self.root_moves.len() > 1 && rng.unit() < self.limits.blunder_chance {
            let pool = self.root_moves.len().min(3);
            choice = Some(self.root_moves[rng.below(pool as u32) as usize].0);
        }
        self.result = choice;
        choice
    }

    fn search_root_move(&mut self, mv: Move, clock: &dyn Fn() -> f64) -> i32 {
        let mut next = self.root.clone();
        next.make_move(mv);
        self.path.clear();
        self.path.push(self.root.hash());
        let depth = self.depth as i32 - 1;
        -self.negamax(&next, depth, -INF, -self.alpha, 1, clock)
    }

    fn ctx(&self) -> Ctx<'_> {
        Ctx { terrain: &self.terrain, rules: &self.rules }
    }

    fn tick(&mut self, clock: &dyn Fn() -> f64) -> bool {
        self.nodes += 1;
        if self.nodes.is_multiple_of(CLOCK_EVERY) && clock() > self.deadline_ms {
            self.aborted = true;
        }
        self.aborted
    }

    fn negamax(
        &mut self,
        pos: &Position,
        mut depth: i32,
        mut alpha: i32,
        mut beta: i32,
        ply: usize,
        clock: &dyn Fn() -> f64,
    ) -> i32 {
        if self.tick(clock) {
            return 0;
        }
        let hash = pos.hash();
        if pos.halfmove_clock >= 100 || self.path.contains(&hash) || self.history.contains(&hash) {
            return 0;
        }
        let side = pos.side_to_move;
        let in_check = self.ctx().in_check(pos, side);
        if in_check && ply < MAX_PLY / 2 {
            depth += 1;
        }
        if depth <= 0 || ply >= MAX_PLY - 1 {
            return self.quiesce(pos, alpha, beta, ply, clock);
        }

        let alpha_orig = alpha;
        let tt_move = match self.tt.probe(hash) {
            Some(e) => {
                if e.depth as i32 >= depth {
                    let score = from_tt(e.score, ply);
                    match e.bound {
                        Bound::Exact => return score,
                        Bound::Lower => alpha = alpha.max(score),
                        Bound::Upper => beta = beta.min(score),
                    }
                    if alpha >= beta {
                        return score;
                    }
                }
                e.mv
            }
            None => None,
        };

        let mut moves = Vec::with_capacity(48);
        self.ctx().pseudo_legal(pos, &mut moves);
        self.order(pos, &mut moves, tt_move, ply);

        self.path.push(hash);
        let mut best = -INF;
        let mut best_move = None;
        let mut legal = 0;
        for mv in moves {
            let mut next = pos.clone();
            next.make_move(mv);
            if self.ctx().in_check(&next, side) {
                continue;
            }
            legal += 1;
            let score = -self.negamax(&next, depth - 1, -beta, -alpha, ply + 1, clock);
            if self.aborted {
                self.path.pop();
                return 0;
            }
            if score > best {
                best = score;
                best_move = Some(mv);
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                if !is_capture(pos, &mv) {
                    let k = &mut self.killers[ply];
                    if k[0] != Some(mv) {
                        k[1] = k[0];
                        k[0] = Some(mv);
                    }
                }
                break;
            }
        }
        self.path.pop();

        if legal == 0 {
            return if in_check { -MATE + ply as i32 } else { 0 };
        }
        let bound = if best <= alpha_orig {
            Bound::Upper
        } else if best >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.tt.store(Entry { key: hash, mv: best_move, score: to_tt(best, ply), depth: depth as i8, bound });
        best
    }

    /// Only captures and promotions, until the position is quiet.
    fn quiesce(
        &mut self,
        pos: &Position,
        mut alpha: i32,
        beta: i32,
        ply: usize,
        clock: &dyn Fn() -> f64,
    ) -> i32 {
        if self.tick(clock) {
            return 0;
        }
        let stand_pat = evaluate(&self.ctx(), pos);
        if stand_pat >= beta || ply >= MAX_PLY - 1 {
            return stand_pat;
        }
        alpha = alpha.max(stand_pat);

        let mut moves = Vec::with_capacity(16);
        self.ctx().pseudo_legal(pos, &mut moves);
        moves.retain(|m| is_capture(pos, m) || m.promotion == Some(tc_core::PieceKind::Queen));
        self.order(pos, &mut moves, None, ply);

        let side = pos.side_to_move;
        for mv in moves {
            let mut next = pos.clone();
            next.make_move(mv);
            if self.ctx().in_check(&next, side) {
                continue;
            }
            let score = -self.quiesce(&next, -beta, -alpha, ply + 1, clock);
            if self.aborted {
                return 0;
            }
            if score >= beta {
                return score;
            }
            alpha = alpha.max(score);
        }
        alpha
    }

    /// TT move, then captures by most valuable victim / least valuable attacker,
    /// then killer moves, then the rest.
    fn order(&self, pos: &Position, moves: &mut [Move], tt_move: Option<Move>, ply: usize) {
        let killers = self.killers[ply.min(MAX_PLY - 1)];
        moves.sort_by_cached_key(|m| {
            if Some(*m) == tt_move {
                return i32::MIN;
            }
            let mut key = 0;
            if let Some(victim) = pos.get(m.to) {
                let attacker = pos.get(m.from).map_or(0, |p| piece_value(p.kind));
                key -= 10_000 + piece_value(victim.kind) * 10 - attacker / 10;
            } else if m.kind == MoveKind::EnPassant {
                key -= 10_000 + 1000;
            } else if killers.contains(&Some(*m)) {
                key -= 5_000;
            }
            if let Some(p) = m.promotion {
                key -= piece_value(p);
            }
            key
        });
    }
}

fn is_capture(pos: &Position, mv: &Move) -> bool {
    pos.get(mv.to).is_some() || mv.kind == MoveKind::EnPassant
}

/// Mate scores are stored relative to the node, not the root.
fn to_tt(score: i32, ply: usize) -> i32 {
    if is_mate(score) { score + score.signum() * ply as i32 } else { score }
}

fn from_tt(score: i32, ply: usize) -> i32 {
    if is_mate(score) { score - score.signum() * ply as i32 } else { score }
}
