use crate::encounter::Encounter;
use crate::map::{CampKind, MapPos, ObjectKind};
use crate::path::tile_cost;
use crate::resolve::{army_value, auto_resolve, can_auto_resolve_win};
use crate::world::World;

impl World {
    /// Spends an overworld turn for an AI hero, including deterministic camp battles.
    pub fn ai_turn(&mut self, hero_id: u8) -> Option<Encounter> {
        if !self.hero(hero_id).is_some_and(|hero| hero.alive && hero.is_ai) {
            return None;
        }

        let max_steps =
            self.map.tiles.len().min(256).saturating_add(self.map.objects.len()).saturating_add(1);
        for _ in 0..max_steps {
            if !self.hero(hero_id).is_some_and(|hero| hero.alive && hero.movement > 0.0) {
                break;
            }

            let Some(player_id) =
                self.heroes.iter().find(|hero| hero.alive && !hero.is_ai).map(|hero| hero.id)
            else {
                break;
            };
            let ai_value = army_value(&self.hero(hero_id).unwrap().roster);
            let player_value = army_value(&self.hero(player_id).unwrap().roster);
            let hunt = ai_value as f32 >= 1.5 * player_value as f32 + 2.0;

            let target = if hunt {
                Some(self.hero(player_id).unwrap().pos)
            } else {
                self.best_reachable_camp(hero_id)
                    .or_else(|| self.nearest_reachable_pickup(hero_id))
                    .or_else(|| self.nearest_unrevealed(hero_id))
                    .or_else(|| self.nearest_to_centre(hero_id))
            };
            let Some(target) = target else { break };
            let old_pos = self.hero(hero_id).unwrap().pos;
            let encounter = self.move_hero(hero_id, target);

            match encounter {
                Some(Encounter::Hero(other_id)) => return Some(Encounter::Hero(other_id)),
                Some(Encounter::Camp(camp)) => {
                    let guards = self.guards_for(&camp.guards, camp.tier);
                    let roster = self.hero(hero_id).unwrap().roster.clone();
                    if !can_auto_resolve_win(&roster, &guards) {
                        let hero = self.hero_mut(hero_id).unwrap();
                        hero.pos = hero.prev_pos;
                        hero.movement = 0.0;
                        break;
                    }
                    let mut rng = self.next_rng();
                    let result = auto_resolve(&roster, &guards, &mut rng);
                    let won = result.outcome == crate::encounter::Outcome::Won;
                    self.apply_battle(hero_id, Encounter::Camp(camp), result);
                    if !won {
                        break;
                    }
                }
                None => {}
            }

            if self.hero(hero_id).unwrap().pos == old_pos {
                break;
            }
        }

        None
    }

    fn best_reachable_camp(&self, hero_id: u8) -> Option<MapPos> {
        let hero = self.hero(hero_id)?;
        let army = army_value(&hero.roster);
        let mut best: Option<(f32, MapPos)> = None;

        for object in &self.map.objects {
            let ObjectKind::Camp(kind) = object.kind else { continue };
            if object.cleared || object.owner.is_some() {
                continue;
            }
            let guards = self.guards_for(&object.guards, object.tier);
            if army_value(&guards) > army || !can_auto_resolve_win(&hero.roster, &guards) {
                continue;
            }
            let Some(path) = self.path(hero_id, object.pos) else { continue };
            let Some(cost) = path_cost(&self.map, &path) else { continue };
            let score = camp_value(kind) as f32 / (cost + 1.0);
            if best.is_none_or(|(best_score, _)| score > best_score) {
                best = Some((score, object.pos));
            }
        }

        best.map(|(_, pos)| pos)
    }

    fn nearest_reachable_pickup(&self, hero_id: u8) -> Option<MapPos> {
        self.hero(hero_id)?;
        let mut best: Option<(f32, MapPos)> = None;

        for object in &self.map.objects {
            if object.cleared
                || object.owner.is_some()
                || !matches!(object.kind, ObjectKind::Chest | ObjectKind::Shrine)
            {
                continue;
            }
            let Some(path) = self.path(hero_id, object.pos) else { continue };
            let Some(cost) = path_cost(&self.map, &path) else { continue };
            if best.is_none_or(|(best_cost, _)| cost < best_cost) {
                best = Some((cost, object.pos));
            }
        }

        best.map(|(_, pos)| pos)
    }

    fn nearest_unrevealed(&self, hero_id: u8) -> Option<MapPos> {
        let hero = self.hero(hero_id)?;
        let mut candidates = Vec::new();
        for y in 0..self.map.size.1 {
            for x in 0..self.map.size.0 {
                let pos = MapPos::new(x, y);
                if !self.is_revealed(pos) && tile_cost(&self.map, pos).is_some() {
                    candidates.push(pos);
                }
            }
        }
        candidates.sort_by_key(|pos| (hero.pos.manhattan(*pos), pos.y, pos.x));
        candidates.into_iter().find(|pos| self.path(hero_id, *pos).is_some())
    }

    fn nearest_to_centre(&self, hero_id: u8) -> Option<MapPos> {
        let hero = self.hero(hero_id)?;
        let centre = MapPos::new(self.map.size.0 / 2, self.map.size.1 / 2);
        let mut candidates: Vec<_> = (0..self.map.size.1)
            .flat_map(|y| (0..self.map.size.0).map(move |x| MapPos::new(x, y)))
            .filter(|pos| tile_cost(&self.map, *pos).is_some())
            .collect();
        candidates.sort_by_key(|pos| (centre.manhattan(*pos), hero.pos.manhattan(*pos), pos.y, pos.x));
        candidates.into_iter().find(|pos| self.path(hero_id, *pos).is_some())
    }
}

fn path_cost(map: &crate::map::WorldMap, path: &[MapPos]) -> Option<f32> {
    path.iter().try_fold(0.0, |total, pos| Some(total + tile_cost(map, *pos)?))
}

fn camp_value(kind: CampKind) -> u8 {
    match kind {
        CampKind::Village => 2,
        CampKind::KnightCamp | CampKind::BishopCamp => 3,
        CampKind::Fortress => 5,
        CampKind::Citadel => 9,
    }
}
