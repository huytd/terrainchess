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
    cost: f32,
    pos: MapPos,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn find_path(map: &WorldMap, start: MapPos, end: MapPos) -> Option<Vec<MapPos>> {
    let mut heap = BinaryHeap::new();
    let mut dist = HashMap::new();
    let mut came_from = HashMap::new();

    heap.push(State { cost: 0.0, pos: start });
    dist.insert(start, 0.0f32);

    while let Some(State { cost, pos }) = heap.pop() {
        if pos == end {
            let mut path = Vec::new();
            let mut current = pos;
            while current != start {
                path.push(current);
                current = came_from[&current];
            }
            path.reverse();
            return Some(path);
        }

        let neighbors = [
            MapPos::new(pos.x.saturating_sub(1), pos.y),
            MapPos::new(pos.x + 1, pos.y),
            MapPos::new(pos.x, pos.y.saturating_sub(1)),
            MapPos::new(pos.x, pos.y + 1),
        ];

        for &n in &neighbors {
            if n.x >= map.size.0 || n.y >= map.size.1 || n == pos {
                continue;
            }
            if let Some(c) = tile_cost(map, n) {
                let next_cost = cost + c;
                if next_cost < *dist.get(&n).unwrap_or(&f32::INFINITY) {
                    dist.insert(n, next_cost);
                    came_from.insert(n, pos);
                    heap.push(State { cost: next_cost + n.manhattan(end) as f32, pos: n });
                }
            }
        }
    }
    None
}
