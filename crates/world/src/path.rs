use crate::map::{Biome, MapPos, WorldMap};
use std::collections::{BinaryHeap, HashMap};

pub fn tile_cost(map: &WorldMap, pos: MapPos) -> Option<f32> {
    let tile = map.get(pos)?;
    if tile.biome == Biome::Mountain || (tile.biome == Biome::Water && !tile.road) {
        return None;
    }
    if tile.road {
        return Some(0.5);
    }
    match tile.biome {
        Biome::Forest | Biome::Hills => Some(2.0),
        _ => Some(1.0),
    }
}

#[derive(Clone, Copy, PartialEq)]
struct State {
    /// Path cost so far plus the heuristic; orders the heap.
    f: f32,
    /// Path cost so far.
    g: f32,
    node: (MapPos, u8),
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.f.partial_cmp(&self.f).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Direction index of the step into a node; `NO_DIR` for the start.
const NO_DIR: u8 = 4;

/// A* over the 4-neighbour grid. `step(prev_dir, dir, to)` returns the cost of stepping onto `to`
/// in direction `dir` (0..4) after arriving by `prev_dir`, or `None` if blocked. `h_scale` must
/// not exceed the cheapest possible step so the manhattan heuristic stays admissible.
pub fn astar(
    map: &WorldMap,
    start: MapPos,
    end: MapPos,
    h_scale: f32,
    step: impl Fn(u8, u8, MapPos) -> Option<f32>,
) -> Option<Vec<MapPos>> {
    let mut heap = BinaryHeap::new();
    let mut best: HashMap<(MapPos, u8), f32> = HashMap::new();
    let mut came_from: HashMap<(MapPos, u8), (MapPos, u8)> = HashMap::new();
    let start_node = (start, NO_DIR);
    heap.push(State { f: 0.0, g: 0.0, node: start_node });
    best.insert(start_node, 0.0);

    while let Some(State { g, node, .. }) = heap.pop() {
        let (pos, dir) = node;
        if pos == end {
            let mut path = Vec::new();
            let mut current = node;
            while current != start_node {
                path.push(current.0);
                current = came_from[&current];
            }
            path.reverse();
            return Some(path);
        }
        if g > *best.get(&node).unwrap_or(&f32::INFINITY) {
            continue;
        }
        let x = pos.x as i32;
        let y = pos.y as i32;
        for (d, (nx, ny)) in [(x, y - 1), (x + 1, y), (x, y + 1), (x - 1, y)].into_iter().enumerate() {
            if nx < 0 || ny < 0 || nx >= map.size.0 as i32 || ny >= map.size.1 as i32 {
                continue;
            }
            let n = MapPos::new(nx as u16, ny as u16);
            let Some(c) = step(dir, d as u8, n) else { continue };
            let next = (n, d as u8);
            let next_g = g + c;
            if next_g < *best.get(&next).unwrap_or(&f32::INFINITY) {
                best.insert(next, next_g);
                came_from.insert(next, node);
                heap.push(State { f: next_g + h_scale * n.manhattan(end) as f32, g: next_g, node: next });
            }
        }
    }
    None
}

/// Cheapest path for a hero from `start` to `end` (excluding `start`), by `tile_cost`.
pub fn find_path(map: &WorldMap, start: MapPos, end: MapPos) -> Option<Vec<MapPos>> {
    astar(map, start, end, 0.5, |_, _, n| tile_cost(map, n))
}
