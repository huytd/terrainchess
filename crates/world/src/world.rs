use serde::{Deserialize, Serialize};
use tc_core::piece::{PieceKind, Side};
use tc_core::rng::Rng;
use tc_core::worldgen::GenParams;
use tc_run::MatchSetup;

use crate::encounter::{BattleResult, Encounter, Outcome};
use crate::hero::{Hero, HeroId};
use crate::map::{Biome, CampKind, MapPos, ObjectKind, WorldMap};
use crate::path::find_path;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldParams {
    pub width: u16,
    pub height: u16,
    pub heroes: u8,
    pub camps_per_region: u8,
    pub hero_base_movement: f32,
    pub sight_radius: u16,
    pub guard_scaling_rate: f32,
}

impl Default for WorldParams {
    fn default() -> Self {
        WorldParams {
            width: 48,
            height: 48,
            heroes: 2,
            camps_per_region: 12,
            hero_base_movement: 10.0,
            sight_radius: 5,
            guard_scaling_rate: 0.1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub map: WorldMap,
    pub heroes: Vec<Hero>,
    pub day: u32,
    pub turn_order: Vec<HeroId>,
    pub current: usize,
    pub fog: Vec<u64>, // Bitset for fog (1 bit per tile)
    pub rng_state: u64,
    pub params: WorldParams,
}

impl World {
    pub fn new(seed: u64, params: WorldParams) -> Self {
        let (map, heroes) = crate::generator::generate_world(seed, params.clone());
        let mut turn_order = Vec::new();
        for h in &heroes {
            turn_order.push(h.id);
        }

        let fog_words = (map.size.0 as usize * map.size.1 as usize).div_ceil(64);
        let mut world = World {
            map,
            heroes,
            day: 1,
            turn_order,
            current: 0,
            fog: vec![0; fog_words],
            rng_state: seed,
            params,
        };

        for id in world.turn_order.clone() {
            world.reveal_fog(id);
        }
        world.refill_movement();
        world
    }

    pub fn next_rng(&mut self) -> Rng {
        let mut r = Rng::new(self.rng_state);
        self.rng_state = r.next_u64();
        r
    }

    pub fn refill_movement(&mut self) {
        for h in &mut self.heroes {
            if h.alive {
                h.movement = self.params.hero_base_movement;
            }
        }
    }

    pub fn reveal_fog(&mut self, hero_id: HeroId) {
        let hero = self.hero(hero_id).unwrap();
        let center = hero.pos;
        let r = self.params.sight_radius as i32;

        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let nx = center.x as i32 + dx;
                    let ny = center.y as i32 + dy;
                    if nx >= 0 && nx < self.map.size.0 as i32 && ny >= 0 && ny < self.map.size.1 as i32 {
                        let idx = ny as usize * self.map.size.0 as usize + nx as usize;
                        self.fog[idx / 64] |= 1 << (idx % 64);
                    }
                }
            }
        }
    }

    pub fn is_revealed(&self, pos: MapPos) -> bool {
        let idx = pos.y as usize * self.map.size.0 as usize + pos.x as usize;
        (self.fog[idx / 64] & (1 << (idx % 64))) != 0
    }

    pub fn hero(&self, id: HeroId) -> Option<&Hero> {
        self.heroes.iter().find(|h| h.id == id)
    }

    pub fn hero_mut(&mut self, id: HeroId) -> Option<&mut Hero> {
        self.heroes.iter_mut().find(|h| h.id == id)
    }

    pub fn end_turn(&mut self) {
        self.current += 1;
        if self.current >= self.turn_order.len() {
            self.current = 0;
            self.day += 1;
            self.refill_movement();
        }
        while !self.heroes.iter().find(|h| h.id == self.turn_order[self.current]).unwrap().alive {
            self.current += 1;
            if self.current >= self.turn_order.len() {
                self.current = 0;
                self.day += 1;
                self.refill_movement();
            }
        }
    }

    pub fn guards_for(&self, guards: &[PieceKind], tier: u8) -> Vec<PieceKind> {
        let mut scaled = guards.to_vec();
        // Base additions from tier
        for _ in 1..tier {
            scaled.push(PieceKind::Pawn);
        }

        let extra = (self.day as f32 * self.params.guard_scaling_rate) as usize;
        for i in 0..extra {
            if i > 0 && i % 3 == 0 {
                // Upgrade a pawn to a Knight/Bishop at higher values
                if let Some(pos) = scaled.iter().position(|&p| p == PieceKind::Pawn) {
                    scaled[pos] = if i % 2 == 0 { PieceKind::Knight } else { PieceKind::Bishop };
                } else {
                    scaled.push(PieceKind::Pawn);
                }
            } else {
                scaled.push(PieceKind::Pawn);
            }
        }
        scaled
    }

    pub fn path(&self, hero_id: HeroId, dest: MapPos) -> Option<Vec<MapPos>> {
        let hero = self.hero(hero_id)?;
        find_path(&self.map, hero.pos, dest)
    }

    pub fn move_hero(&mut self, hero_id: HeroId, dest: MapPos) -> Option<Encounter> {
        let path = self.path(hero_id, dest)?;
        let mut encounter = None;

        for p in path {
            if let Some(c) = crate::path::tile_cost(&self.map, p) {
                let hero = self.hero_mut(hero_id).unwrap();
                if hero.movement >= c {
                    hero.movement -= c;
                    hero.prev_pos = hero.pos;
                    hero.pos = p;
                    self.reveal_fog(hero_id);

                    let mut immediate_reward = false;
                    if let Some(obj) = self.map.get_object(p)
                        && !obj.cleared
                    {
                        match obj.kind {
                            ObjectKind::Camp(_) => {
                                encounter = Some(Encounter::Camp(obj.clone()));
                                break;
                            }
                            ObjectKind::Chest => {
                                immediate_reward = true;
                                let mut r = self.next_rng();
                                // Random card, e.g. from filler deck
                                let deck = tc_core::filler_deck(r.next_u64());
                                if let Some(&card) = deck.first() {
                                    let hero = self.hero_mut(hero_id).unwrap();
                                    hero.cards.push(card);
                                }
                            }
                            ObjectKind::Shrine => {
                                immediate_reward = true;
                                let hero = self.hero_mut(hero_id).unwrap();
                                hero.movement += 3.0;
                            }
                            ObjectKind::Signpost => {
                                // Nothing
                            }
                        }
                    }
                    if immediate_reward {
                        let obj = self.map.get_object_mut(p).unwrap();
                        obj.cleared = true;
                    }
                    if let Some(other) = self.heroes.iter().find(|h| h.id != hero_id && h.alive && h.pos == p)
                    {
                        encounter = Some(Encounter::Hero(other.id));
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        encounter
    }

    pub fn battle_setup(&mut self, hero_id: HeroId, encounter: &Encounter) -> MatchSetup {
        let seed = self.next_rng().next_u64();
        let hero = self.hero(hero_id).unwrap();

        let mut setup = MatchSetup {
            seed,
            r#gen: GenParams::for_floor(8, 0),
            rules: tc_core::rules::Rules::standard(8),
            ai_level: 3,
            player: if !hero.is_ai { Side::White } else { Side::Black },
            relics: Vec::new(),
            pickups: Vec::new(),
            player_deck: hero.cards.clone(),
            enemy_deck: Vec::new(),
            enemy_items: Vec::new(),
            veteran: [false, false],
            armies: None,
        };

        if setup.player_deck.len() < 15 {
            setup.player_deck = tc_core::filler_deck(seed);
        }

        match encounter {
            Encounter::Camp(obj) => {
                let guards = self.guards_for(&obj.guards, obj.tier);
                setup.armies = Some([hero.roster.clone(), guards]);
                setup.enemy_deck = tc_core::filler_deck(seed ^ 0x454E_454D_595F_4445);

                let tile = self.map.get(obj.pos).unwrap();
                match tile.biome {
                    Biome::Forest => setup.r#gen.obstacle_density += 0.1,
                    Biome::Hills | Biome::Mountain => setup.r#gen.roughness += 0.5,
                    Biome::Coast => setup.r#gen.basins += 1.0,
                    Biome::Grass => setup.r#gen.roughness *= 0.5,
                    _ => {}
                };
            }
            Encounter::Hero(other_id) => {
                let other = self.hero(*other_id).unwrap();
                if !other.is_ai {
                    setup.player = Side::Black;
                }
                setup.armies = Some([hero.roster.clone(), other.roster.clone()]);
                setup.enemy_deck = other.cards.clone();
                if setup.enemy_deck.len() < 15 {
                    setup.enemy_deck = tc_core::filler_deck(seed ^ 0x454E_454D_595F_4445);
                }
            }
        }
        setup
    }

    pub fn apply_battle(&mut self, hero_id: HeroId, encounter: Encounter, result: BattleResult) {
        let hero = self.hero_mut(hero_id).unwrap();

        // Only remove pieces if not retreated? Actually if retreated we still lose what died.
        let mut survivor_roster = hero.roster.clone();
        for lost in &result.lost {
            if let Some(pos) = survivor_roster.iter().position(|p| p == lost) {
                survivor_roster.remove(pos);
            }
        }
        hero.roster = survivor_roster;

        match encounter {
            Encounter::Camp(obj) => {
                if result.outcome == Outcome::Won {
                    let map_obj = self.map.get_object_mut(obj.pos).unwrap();
                    map_obj.cleared = true;
                    map_obj.owner = Some(hero_id);
                    let kind = map_obj.kind.clone();

                    let mut r = self.next_rng();
                    let boss = match kind {
                        ObjectKind::Camp(CampKind::Village) => {
                            let extra_pawns = 1 + (r.next_u64() % 2);
                            for _ in 0..extra_pawns {
                                self.hero_mut(hero_id).unwrap().roster.push(PieceKind::Pawn);
                            }
                            PieceKind::Pawn
                        }
                        ObjectKind::Camp(CampKind::KnightCamp) => PieceKind::Knight,
                        ObjectKind::Camp(CampKind::BishopCamp) => PieceKind::Bishop,
                        ObjectKind::Camp(CampKind::Fortress) => PieceKind::Rook,
                        ObjectKind::Camp(CampKind::Citadel) => PieceKind::Queen,
                        _ => PieceKind::Pawn,
                    };
                    if let ObjectKind::Camp(CampKind::Village) = kind {
                        // Already gave pawns
                    } else {
                        self.hero_mut(hero_id).unwrap().roster.push(boss);
                    }
                } else if result.outcome == Outcome::Retreated {
                    let map_obj = self.map.get_object_mut(obj.pos).unwrap();
                    let mut remaining_guards = obj.guards.clone();
                    for lost in result.enemy_lost {
                        if let Some(pos) = remaining_guards.iter().position(|&p| p == lost) {
                            remaining_guards.remove(pos);
                        }
                    }
                    map_obj.guards = remaining_guards;
                    let hero = self.hero_mut(hero_id).unwrap();
                    hero.pos = hero.prev_pos;
                    hero.movement = 0.0;
                } else {
                    self.hero_mut(hero_id).unwrap().alive = false;
                }
            }
            Encounter::Hero(other_id) => {
                if result.outcome == Outcome::Won {
                    self.hero_mut(other_id).unwrap().alive = false;
                } else if result.outcome == Outcome::Retreated {
                    let hero = self.hero_mut(hero_id).unwrap();
                    hero.pos = hero.prev_pos;
                    hero.movement = 0.0;
                } else {
                    self.hero_mut(hero_id).unwrap().alive = false;
                }
            }
        }
    }

    pub fn winner(&self) -> Option<HeroId> {
        let alive: Vec<_> = self.heroes.iter().filter(|h| h.alive).collect();
        if alive.len() == 1 { Some(alive[0].id) } else { None }
    }

    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    pub fn from_ron(s: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(s)
    }
}
