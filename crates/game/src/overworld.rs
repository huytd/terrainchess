#![allow(unused)]
#![allow(
    clippy::too_many_arguments,
    clippy::collapsible_if,
    clippy::get_first,
    clippy::needless_return,
    clippy::type_complexity
)]
use bevy::asset::RenderAssetUsages;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::touch::Touches;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::atlas::Atlas;
use crate::board_view::{
    Look, Overlay, PX, PickupSprite, PieceShadow, PieceSilhouette, PieceSprite, Quads, TerrainPart,
    ThreatBadge, card_mesh, flat, full_uv, hash, rotate_uv, square_top,
};
use crate::game::{GameState, Mode};
use crate::hud::{ButtonDisabled, ButtonVisuals, INK_WOOD, button_slicer, panel_slicer};
use crate::input::{MainCamera, OVERWORLD_START_DISTANCE, Orbit, TAP_SLOP};
use crate::loading::AppState;
use crate::run::{Campaign, TitleMenu};
use crate::scenery::{Bird, SceneryPart};

use tc_core::piece::PieceKind;
use tc_world::map::{Biome, CampKind, MapTile, ObjectKind, WorldMap};
use tc_world::{MapPos, encounter::Encounter};

const ROAD_TILES: [&str; 16] = [
    "ow_road_00",
    "ow_road_01",
    "ow_road_02",
    "ow_road_03",
    "ow_road_04",
    "ow_road_05",
    "ow_road_06",
    "ow_road_07",
    "ow_road_08",
    "ow_road_09",
    "ow_road_0a",
    "ow_road_0b",
    "ow_road_0c",
    "ow_road_0d",
    "ow_road_0e",
    "ow_road_0f",
];

const BRIDGE_TILES: [&str; 16] = [
    "ow_bridge_00",
    "ow_bridge_01",
    "ow_bridge_02",
    "ow_bridge_03",
    "ow_bridge_04",
    "ow_bridge_05",
    "ow_bridge_06",
    "ow_bridge_07",
    "ow_bridge_08",
    "ow_bridge_09",
    "ow_bridge_0a",
    "ow_bridge_0b",
    "ow_bridge_0c",
    "ow_bridge_0d",
    "ow_bridge_0e",
    "ow_bridge_0f",
];

const COAST_TILES: [&str; 16] = [
    "ow_coast_00",
    "ow_coast_01",
    "ow_coast_02",
    "ow_coast_03",
    "ow_coast_04",
    "ow_coast_05",
    "ow_coast_06",
    "ow_coast_07",
    "ow_coast_08",
    "ow_coast_09",
    "ow_coast_0a",
    "ow_coast_0b",
    "ow_coast_0c",
    "ow_coast_0d",
    "ow_coast_0e",
    "ow_coast_0f",
];

/// One seamless overworld tile texture per biome, picked by a position hash so
/// neighbouring tiles of the same biome don't look identical.
fn biome_tile(biome: Biome, x: u16, y: u16) -> &'static str {
    let h = hash(x as i32, y as i32, 501) as usize;
    match biome {
        Biome::Grass => ["ow_grass_0", "ow_grass_1", "ow_grass_2"][h % 3],
        Biome::Forest => ["ow_forest_0", "ow_forest_1", "ow_forest_2", "ow_forest_3", "ow_forest_4"][h % 5],
        Biome::Hills => ["ow_hills_0", "ow_hills_1", "ow_hills_2"][h % 3],
        Biome::Mountain => ["ow_mountain_0", "ow_mountain_1", "ow_mountain_2", "ow_mountain_3"][h % 4],
        Biome::Water => ["ow_water_0", "ow_water_1", "ow_water_2"][h % 3],
        Biome::Coast => "ow_coast_00",
    }
}

/// Pick the coast transition whose land edges match adjacent non-water biomes.
fn coast_tile(map: &WorldMap, pos: MapPos) -> &'static str {
    let directions = [(0, -1, 1), (1, 0, 2), (0, 1, 4), (-1, 0, 8)];
    let mut mask = 0;
    for (dx, dy, bit) in directions {
        let nx = pos.x as i32 + dx;
        let ny = pos.y as i32 + dy;
        if nx >= 0
            && ny >= 0
            && map.get(MapPos::new(nx as u16, ny as u16)).is_some_and(|tile| tile.biome != Biome::Water)
        {
            mask |= bit;
        }
    }
    COAST_TILES[mask]
}

/// The road (or bridge, over water) sprite for the tile's N/E/S/W road mask.
fn road_tile(map: &WorldMap, pos: MapPos, is_water: bool) -> (&'static str, u32) {
    let has_road = |dx: i32, dy: i32| -> bool {
        let nx = pos.x as i32 + dx;
        let ny = pos.y as i32 + dy;
        if nx < 0 || ny < 0 {
            return false;
        }
        map.get(MapPos::new(nx as u16, ny as u16)).is_some_and(|t: MapTile| t.road)
    };
    let mask = (has_road(0, -1) as usize)
        | ((has_road(1, 0) as usize) << 1)
        | ((has_road(0, 1) as usize) << 2)
        | ((has_road(-1, 0) as usize) << 3);
    (if is_water { BRIDGE_TILES[mask] } else { ROAD_TILES[mask] }, 0)
}

/// Rotate the footstep art's top-left-to-bottom-right diagonal along this path tile.
fn path_marker_rotation(from: MapPos, current: MapPos, to: MapPos) -> Quat {
    let incoming = Vec2::new(current.x as f32 - from.x as f32, current.y as f32 - from.y as f32);
    let outgoing = Vec2::new(to.x as f32 - current.x as f32, to.y as f32 - current.y as f32);
    let direction = incoming + outgoing;
    let yaw = direction.x.atan2(direction.y) - std::f32::consts::FRAC_PI_4;
    Quat::from_rotation_y(yaw)
}

pub struct OverworldPlugin;

impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverworldState>()
            .init_resource::<OverworldMouseGesture>()
            .init_resource::<OverworldTouchGesture>()
            .add_systems(OnEnter(Mode::Overworld), (hide_classic, setup_overworld).chain())
            .add_systems(
                Update,
                (
                    update_overworld_map,
                    update_overworld_hud,
                    overworld_pointer,
                    handle_overworld_input,
                    animate_hero,
                )
                    .chain()
                    .run_if(in_state(Mode::Overworld)),
            )
            .add_systems(OnExit(Mode::Overworld), teardown_overworld)
            .add_systems(OnEnter(Mode::Classic), show_classic)
            .add_systems(OnEnter(Mode::Deploy), show_classic)
            .add_systems(OnEnter(Mode::OverworldBattle), (show_classic, setup_overworld_battle_hud))
            .add_systems(OnExit(Mode::OverworldBattle), teardown_overworld_battle_hud)
            .add_systems(
                Update,
                (
                    handle_deploy,
                    handle_deck_interactions,
                    update_deck_ui,
                    scroll_deck_list,
                    update_deploy_start,
                    handle_deploy_input,
                )
                    .chain()
                    .run_if(in_state(Mode::Deploy)),
            )
            .add_systems(Update, handle_battle.run_if(in_state(Mode::OverworldBattle)))
            .add_systems(Update, update_retreat_button_visibility.run_if(in_state(Mode::OverworldBattle)))
            .add_systems(OnEnter(Mode::Deploy), setup_deploy)
            .add_systems(OnExit(Mode::Deploy), teardown_deploy)
            .add_systems(Update, handle_reward_pick.run_if(in_state(AppState::Ready)));
    }
}

fn hide_classic(
    mut q: Query<
        &mut Visibility,
        Or<(
            With<TerrainPart>,
            With<Overlay>,
            With<PieceSprite>,
            With<PieceShadow>,
            With<PickupSprite>,
            With<PieceSilhouette>,
            With<SceneryPart>,
            With<Bird>,
            With<ThreatBadge>,
        )>,
    >,
) {
    for mut vis in q.iter_mut() {
        *vis = Visibility::Hidden;
    }
}

fn setup_overworld_battle_hud(mut commands: Commands, atlas: Res<Atlas>) {
    // Anchored top-right, below the round menu button and well above the hand
    // bar / discard button / deck pile which all live at the bottom of the
    // screen, so it never overlaps them.
    commands
        .spawn((
            BattleHudRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(70.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Button,
                Interaction::default(),
                RetreatButton,
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ImageNode {
                    image: atlas.image.clone(),
                    rect: Some(atlas.rect("btn_wood")),
                    image_mode: crate::hud::button_slicer(),
                    ..default()
                },
            ))
            .with_child((
                Text::new("Retreat"),
                TextFont { font_size: FontSize::Px(24.0), ..default() },
                TextColor(crate::hud::INK_WOOD),
            ));
        });
}

fn teardown_overworld_battle_hud(mut commands: Commands, root: Query<Entity, With<BattleHudRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}

/// The retreat button is only meaningful while a campaign battle is actually
/// being played; hide it during the post-battle reward draft (and anywhere
/// else `RunPhase` isn't `Playing`).
fn update_retreat_button_visibility(
    run: Res<crate::run::Run>,
    state: Res<OverworldState>,
    mut q: Query<&mut Visibility, With<RetreatButton>>,
) {
    let show =
        run.phase == crate::run::RunPhase::Playing && !matches!(state.encounter, Some(Encounter::Hero(_)));
    for mut vis in &mut q {
        *vis = if show { Visibility::Inherited } else { Visibility::Hidden };
    }
}

fn show_classic(
    mut q: Query<
        &mut Visibility,
        Or<(
            With<TerrainPart>,
            With<Overlay>,
            With<PieceSprite>,
            With<PieceShadow>,
            With<PickupSprite>,
            With<PieceSilhouette>,
            With<SceneryPart>,
            With<Bird>,
            With<ThreatBadge>,
        )>,
    >,
) {
    for mut vis in q.iter_mut() {
        *vis = Visibility::Inherited;
    }
}

#[derive(Resource)]
struct OverworldState {
    pub dirty: bool,
    pub reward_draft: Option<[String; 3]>,
    pub campaign_over: bool,
    pub campaign_won: bool,
    pub deck_open: bool,
    pub path: Option<Vec<MapPos>>,
    pub moving: bool,
    pub encounter: Option<Encounter>,
    pub selected_tile: Option<MapPos>,
    pub anim_timer: Timer,
    /// True while the hero is walking west/left, so its sprite is mirrored.
    pub facing_left: bool,
}

impl Default for OverworldState {
    fn default() -> Self {
        Self {
            dirty: false,
            path: None,
            moving: false,
            encounter: None,
            selected_tile: None,
            anim_timer: Timer::from_seconds(0.12, TimerMode::Repeating),
            reward_draft: None,
            campaign_over: false,
            campaign_won: false,
            deck_open: false,
            facing_left: false,
        }
    }
}

#[derive(Component)]
struct OverworldRoot;

#[derive(Component)]
struct OverworldHero;

#[derive(Component)]
struct OverworldHudRoot;

#[derive(Resource, Default)]
struct OverworldMouseGesture {
    active: bool,
    dragged: bool,
    start: Vec2,
    last: Vec2,
}

#[derive(Resource, Default)]
struct OverworldTouchGesture {
    active: bool,
    dragged: bool,
}

fn setup_overworld(
    campaign: Option<Res<Campaign>>,
    mut orbit: Query<&mut Orbit>,
    mut state: ResMut<OverworldState>,
    mut title_menu: ResMut<TitleMenu>,
) {
    title_menu.open = false;
    title_menu.pending = false;
    state.dirty = true;
    state.path = None;
    state.moving = false;
    state.encounter = None;
    state.selected_tile = None;

    if let Some(c) = campaign {
        if let Some(mut o) = orbit.iter_mut().next() {
            o.distance = OVERWORLD_START_DISTANCE;
            if let Some(hero) = c.world.heroes.get(0) {
                o.focus = Vec3::new(hero.pos.x as f32, 0.0, hero.pos.y as f32);
            } else {
                let (width, height) = c.world.map.size;
                o.focus = Vec3::new((width as f32 - 1.0) / 2.0, 0.0, (height as f32 - 1.0) / 2.0);
            }
            let (width, height) = c.world.map.size;
            o.clamp_overworld_focus(width, height);
        }
    }
}

fn teardown_overworld(
    mut commands: Commands,
    root: Query<Entity, With<OverworldRoot>>,
    hud_root: Query<Entity, With<OverworldHudRoot>>,
) {
    for e in &root {
        commands.entity(e).despawn();
    }
    for e in &hud_root {
        commands.entity(e).despawn();
    }
}

fn tile_height(biome: Biome) -> f32 {
    match biome {
        Biome::Water => 0.0,
        Biome::Coast => 0.5,
        Biome::Grass => 1.0,
        Biome::Forest => 1.0,
        Biome::Hills => 2.0,
        Biome::Mountain => 3.0,
    }
}

const FOG_SUBDIVISIONS: usize = 4;
const FOG_FADE_DISTANCE: f32 = 1.9;

fn fog_distance(world: &tc_world::World, point: Vec2) -> f32 {
    let radius = FOG_FADE_DISTANCE + 0.5;
    let min_x = ((point.x - radius).floor() as i32).max(0);
    let max_x = ((point.x + radius).ceil() as i32).min(world.map.size.0 as i32 - 1);
    let min_y = ((point.y - radius).floor() as i32).max(0);
    let max_y = ((point.y + radius).ceil() as i32).min(world.map.size.1 as i32 - 1);
    let mut nearest = FOG_FADE_DISTANCE + 1.0;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if !world.is_revealed(MapPos::new(x as u16, y as u16)) {
                continue;
            }

            // Distance to the edge of the revealed tile, rather than its center,
            // keeps known ground clear while letting the fog roll outward from it.
            let dx = ((point.x - x as f32).abs() - 0.5).max(0.0);
            let dy = ((point.y - y as f32).abs() - 0.5).max(0.0);
            nearest = nearest.min(dx.hypot(dy));
        }
    }

    nearest
}

fn fog_noise(x: f32, y: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    let sample = |sx, sy| hash(sx, sy, 0xF09C_31A7) as f32 / u32::MAX as f32 * 2.0 - 1.0;
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let top = lerp(sample(ix, iy), sample(ix + 1, iy), smooth(fx));
    let bottom = lerp(sample(ix, iy + 1), sample(ix + 1, iy + 1), smooth(fx));
    lerp(top, bottom, smooth(fy))
}

fn fog_overlay_mesh(world: &tc_world::World) -> Mesh {
    let width = world.map.size.0 as usize;
    let height = world.map.size.1 as usize;
    let side = FOG_SUBDIVISIONS + 1;
    let vertices_per_tile = side * side;
    let mut positions = Vec::with_capacity(width * height * vertices_per_tile);
    let mut normals = Vec::with_capacity(width * height * vertices_per_tile);
    let mut colors = Vec::with_capacity(width * height * vertices_per_tile);
    let mut indices = Vec::with_capacity(width * height * FOG_SUBDIVISIONS * FOG_SUBDIVISIONS * 6);
    let fog_color = Color::srgb(0.012, 0.015, 0.022).to_linear().to_f32_array();

    for tile_y in 0..height {
        for tile_x in 0..width {
            let pos = MapPos::new(tile_x as u16, tile_y as u16);
            let top = tile_height(world.map.get(pos).unwrap().biome) + 0.025;
            let base = positions.len() as u32;

            for row in 0..=FOG_SUBDIVISIONS {
                for col in 0..=FOG_SUBDIVISIONS {
                    let x = tile_x as f32 - 0.5 + col as f32 / FOG_SUBDIVISIONS as f32;
                    let y = tile_y as f32 - 0.5 + row as f32 / FOG_SUBDIVISIONS as f32;
                    let broad = fog_noise(x * 0.32, y * 0.32);
                    let detail = fog_noise(x * 0.9, y * 0.9);
                    let distance = fog_distance(world, Vec2::new(x, y));
                    let edge_wobble = broad * 0.28 + detail * 0.08;
                    let alpha = if distance == 0.0 {
                        0.0
                    } else {
                        let t = ((distance + edge_wobble) / FOG_FADE_DISTANCE).clamp(0.0, 1.0);
                        let fade = t * t * (3.0 - 2.0 * t);
                        (fade * (0.94 + broad * 0.025 + detail * 0.01)).clamp(0.0, 0.97)
                    };

                    positions.push([x, top, y]);
                    normals.push([0.0, 1.0, 0.0]);
                    colors.push([fog_color[0], fog_color[1], fog_color[2], alpha]);
                }
            }

            for row in 0..FOG_SUBDIVISIONS {
                for col in 0..FOG_SUBDIVISIONS {
                    let a = base + (row * side + col) as u32;
                    let b = a + 1;
                    let d = a + side as u32;
                    let c = d + 1;
                    // Reverse the X/Z grid winding so the top faces point upward.
                    indices.extend([a, c, b, a, d, c]);
                }
            }
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
}

fn update_overworld_map(
    mut commands: Commands,
    campaign: Option<Res<Campaign>>,
    atlas: Res<Atlas>,
    mut look: ResMut<Look>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    root: Query<Entity, With<OverworldRoot>>,
    mut state: ResMut<OverworldState>,
) {
    if !state.dirty {
        return;
    }
    // Note: the flag itself is cleared in `update_overworld_hud`, which runs
    // right after this system in the same `Update` tuple — clearing it here
    // instead would starve that system of the same frame's rebuild.

    for e in &root {
        commands.entity(e).despawn();
    }

    let Some(c) = campaign else { return };
    let root_ent = commands.spawn((OverworldRoot, Transform::default(), Visibility::Inherited)).id();

    let map = &c.world.map;
    let size = map.size;
    let mut solid = Quads::default();
    let mut water = Quads::default();

    // Draw tiles
    for y in 0..size.1 {
        for x in 0..size.0 {
            let pos = MapPos::new(x, y);
            let revealed = c.world.is_revealed(pos);
            let tile = map.get(pos).unwrap();
            let height = tile_height(tile.biome);

            let (name, rot) = if tile.road {
                road_tile(map, pos, tile.biome == Biome::Water)
            } else if tile.biome == Biome::Coast {
                (coast_tile(map, pos), 0)
            } else {
                (biome_tile(tile.biome, x, y), 0)
            };

            let top = Vec3::new(x as f32, height, y as f32);
            let corners = flat(top, Vec2::ONE);
            let uv = rotate_uv(full_uv(atlas.uv(name)), rot);
            let tint = Color::WHITE;

            if tile.biome == Biome::Water {
                water.add_tinted(corners, uv, tint);
            } else {
                solid.add_tinted(corners, uv, tint);
            }

            // Props
            if revealed {
                if tile.biome == Biome::Forest {
                    let prop = "pine";
                    let mesh = card_mesh(&mut look, &mut meshes, &atlas, prop, false);
                    commands.entity(root_ent).with_children(|p| {
                        p.spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(look.cards.clone()),
                            Transform::from_translation(top),
                            crate::board_view::Billboard,
                        ));
                    });
                }
                if tile.biome == Biome::Mountain {
                    let prop = "rock";
                    let mesh = card_mesh(&mut look, &mut meshes, &atlas, prop, false);
                    commands.entity(root_ent).with_children(|p| {
                        p.spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(look.cards.clone()),
                            Transform::from_translation(top),
                            crate::board_view::Billboard,
                        ));
                    });
                }
            }
        }
    }

    let solid_mesh = meshes.add(solid.mesh());
    let water_mesh = meshes.add(water.mesh());
    let fog_mesh = meshes.add(fog_overlay_mesh(&c.world));
    let fog_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        // The fog is a ground tint; sort it behind every transparent billboard.
        depth_bias: -1000.0,
        ..default()
    });
    commands.entity(root_ent).with_children(|parent| {
        parent.spawn((Mesh3d(solid_mesh), MeshMaterial3d(look.terrain.clone())));
        parent.spawn((Mesh3d(water_mesh), MeshMaterial3d(look.terrain.clone())));
        parent.spawn((Mesh3d(fog_mesh), MeshMaterial3d(fog_material)));
    });

    // Objects
    for obj in &map.objects {
        if !c.world.is_revealed(obj.pos) {
            continue;
        }
        if !matches!(obj.kind, ObjectKind::Camp(_)) && obj.cleared {
            continue;
        }

        let sprite = match &obj.kind {
            ObjectKind::Camp(CampKind::Village) => "ow_village",
            ObjectKind::Camp(CampKind::KnightCamp) => "ow_knight_camp",
            ObjectKind::Camp(CampKind::BishopCamp) => "ow_bishop_camp",
            ObjectKind::Camp(CampKind::Fortress) => "ow_fortress",
            ObjectKind::Camp(CampKind::Citadel) => "ow_citadel",
            ObjectKind::Chest => "ow_chest",
            ObjectKind::Shrine => "ow_shrine",
            ObjectKind::Signpost => "ow_signpost",
        };

        // The camp/prop art is already fully coloured, so it's drawn plain white
        // (unlike the chess pieces, which are grayscale and tinted by side); camp
        // ownership shows as a small banner near the building's flag slot instead
        // of retinting the whole building.
        let mesh = card_mesh(&mut look, &mut meshes, &atlas, sprite, false);
        let mut mat = StandardMaterial::from(Color::WHITE);
        mat.base_color_texture = Some(atlas.image.clone());
        mat.alpha_mode = AlphaMode::Blend;
        mat.unlit = true;
        let mat_handle = materials.add(mat);

        let top = Vec3::new(obj.pos.x as f32, tile_height(map.get(obj.pos).unwrap().biome), obj.pos.y as f32);

        commands.entity(root_ent).with_children(|p| {
            p.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_handle),
                Transform::from_translation(top),
                crate::board_view::Billboard,
            ));

            if let ObjectKind::Camp(_) = &obj.kind {
                let banner_sprite = match obj.owner {
                    Some(0) => "ow_banner_sun",
                    Some(_) => "ow_banner_crown",
                    None => "ow_banner_neutral",
                };
                // The building's flag slot sits near its top-right corner.
                let size = atlas.px(sprite) * PX;
                let offset = Vec3::new(size.x * 0.32, size.y * 0.62, -0.05);
                let banner_mesh = card_mesh(&mut look, &mut meshes, &atlas, banner_sprite, false);
                p.spawn((
                    Mesh3d(banner_mesh),
                    MeshMaterial3d(look.cards.clone()),
                    Transform::from_translation(top + offset).with_scale(Vec3::splat(0.4)),
                    crate::board_view::Billboard,
                ));
            }
        });
    }

    // Path preview
    if let Some(sel) = state.selected_tile {
        if let Some(path) = c.world.path(0, sel) {
            let hero = c.world.heroes.first().unwrap();
            let mut cost_so_far = 0.0;

            for (i, p) in path.iter().enumerate() {
                if let Some(cost) = tc_world::path::tile_cost(&c.world.map, *p) {
                    cost_so_far += cost;
                }
                let in_range = cost_so_far <= hero.movement;
                let is_last = i == path.len() - 1;
                let sprite =
                    if is_last { if in_range { "ow_target" } else { "ow_blocked" } } else { "ow_path" };

                let size = atlas.px(sprite) * PX;
                let mut marker = Quads::default();
                marker.add(flat(Vec3::ZERO, size), full_uv(atlas.uv(sprite)), 1.0);
                let mesh = meshes.add(marker.mesh());
                let mut mat = StandardMaterial::from(Color::WHITE);
                mat.base_color_texture = Some(atlas.image.clone());
                mat.alpha_mode = AlphaMode::Blend;
                mat.unlit = true;
                let mat_handle = materials.add(mat);

                let h = tile_height(map.get(*p).unwrap().biome);
                let top = Vec3::new(p.x as f32, h + 0.05, p.y as f32);
                let rotation = if is_last {
                    Quat::IDENTITY
                } else {
                    let from = if i == 0 { hero.pos } else { path[i - 1] };
                    path_marker_rotation(from, *p, path[i + 1])
                };
                commands.entity(root_ent).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(mat_handle),
                        Transform::from_translation(top)
                            .with_rotation(rotation)
                            .with_scale(Vec3::splat(if is_last { 1.3 } else { 0.8 })),
                    ));
                });
            }
        }
    }
    // Heroes: mounted-lord sprites, mirrored via `card_mesh`'s `flip` when
    // walking left.
    for hero in &c.world.heroes {
        if !hero.alive || !c.world.is_revealed(hero.pos) {
            continue;
        }
        let is_player = hero.id == 0;
        let sprite = if is_player { "ow_hero_sun_0" } else { "ow_hero_crown_0" };
        let flip = is_player && state.facing_left;
        let mesh = card_mesh(&mut look, &mut meshes, &atlas, sprite, flip);
        let top =
            Vec3::new(hero.pos.x as f32, tile_height(map.get(hero.pos).unwrap().biome), hero.pos.y as f32);

        let mut ent = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(look.cards.clone()),
            Transform::from_translation(top).with_scale(Vec3::splat(1.2)),
            crate::board_view::Billboard,
        ));

        if is_player {
            ent.insert(OverworldHero);
        }
        let e = ent.id();
        commands.entity(root_ent).add_child(e);
    }
}

fn pick_overworld_tile(c: &Campaign, ray: Ray3d) -> Option<MapPos> {
    let mut best: Option<(f32, MapPos)> = None;
    let size = c.world.map.size;
    for y in 0..size.1 {
        for x in 0..size.0 {
            let pos = MapPos::new(x, y);
            if !c.world.is_revealed(pos) {
                continue;
            }
            let h = tile_height(c.world.map.get(pos).unwrap().biome);

            let lo = Vec3::new(x as f32 - 0.5, -1.0, y as f32 - 0.5);
            let hi = Vec3::new(x as f32 + 0.5, h, y as f32 + 0.5);

            let (mut t0, mut t1) = (f32::NEG_INFINITY, f32::INFINITY);
            for i in 0..3 {
                let d = ray.direction[i];
                let o = ray.origin[i];
                if d.abs() < 1e-6 {
                    if o < lo[i] || o > hi[i] {
                        t0 = f32::INFINITY;
                        break;
                    }
                } else {
                    let mut t_near = (lo[i] - o) / d;
                    let mut t_far = (hi[i] - o) / d;
                    if t_near > t_far {
                        std::mem::swap(&mut t_near, &mut t_far);
                    }
                    t0 = t0.max(t_near);
                    t1 = t1.min(t_far);
                }
            }
            if t0 <= t1 && t0 > 0.0 {
                if best.is_none() || t0 < best.unwrap().0 {
                    best = Some((t0, pos));
                }
            }
        }
    }
    best.map(|b| b.1)
}

fn overworld_pointer(
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    campaign: Option<Res<Campaign>>,
    mut state: ResMut<OverworldState>,
    mut orbit: ResMut<Orbit>,
    mut mouse_gesture: ResMut<OverworldMouseGesture>,
    mut touch_gesture: ResMut<OverworldTouchGesture>,
    ui_query: Query<&Interaction>,
) {
    let Some(campaign) = campaign else {
        *mouse_gesture = OverworldMouseGesture::default();
        *touch_gesture = OverworldTouchGesture::default();
        return;
    };
    let can_select = !state.campaign_over && !state.moving && state.encounter.is_none();
    let on_ui = || ui_query.iter().any(|&interaction| interaction != Interaction::None);

    if mouse.just_pressed(MouseButton::Left) {
        *mouse_gesture = OverworldMouseGesture::default();
        if !on_ui() {
            if let Some(cursor) = window.cursor_position() {
                mouse_gesture.active = true;
                mouse_gesture.start = cursor;
                mouse_gesture.last = cursor;
            }
        }
    }
    if mouse.pressed(MouseButton::Left) && mouse_gesture.active {
        if let Some(cursor) = window.cursor_position() {
            if cursor.distance(mouse_gesture.start) > TAP_SLOP {
                mouse_gesture.dragged = true;
            }
            if mouse_gesture.dragged {
                orbit.pan(cursor - mouse_gesture.last);
            }
            mouse_gesture.last = cursor;
        }
    }
    if mouse.just_released(MouseButton::Left) {
        if mouse_gesture.active && !mouse_gesture.dragged && can_select {
            if let Some(cursor) = window.cursor_position() {
                select_overworld_tile(&campaign, *camera, cursor, &mut state);
            }
        }
        *mouse_gesture = OverworldMouseGesture::default();
    }

    if touches.any_just_pressed() && !touch_gesture.active && !on_ui() {
        *touch_gesture = OverworldTouchGesture { active: true, dragged: false };
    }
    if touch_gesture.active {
        let down: Vec<_> = touches.iter().collect();
        match down.as_slice() {
            [touch] => {
                if touch.distance().length() > TAP_SLOP {
                    touch_gesture.dragged = true;
                }
                if touch_gesture.dragged {
                    orbit.pan(touch.delta());
                }
            }
            [a, b, ..] => {
                touch_gesture.dragged = true;
                let before = b.previous_position() - a.previous_position();
                let now = b.position() - a.position();
                if before.length() > 1.0 && now.length() > 1.0 {
                    orbit.zoom_overworld_by(now.length() / before.length());
                    orbit.turn(before.angle_to(now));
                }
                let avg_delta = (a.delta() + b.delta()) / 2.0;
                orbit.pan(Vec2::new(avg_delta.x, 0.0));
                orbit.tilt(avg_delta.y * 0.005);
            }
            [] => {
                if can_select && !touch_gesture.dragged {
                    if let Some(touch) = touches.iter_just_released().next() {
                        select_overworld_tile(&campaign, *camera, touch.position(), &mut state);
                    }
                }
                *touch_gesture = OverworldTouchGesture::default();
            }
            _ => {}
        }
    }

    let (width, height) = campaign.world.map.size;
    orbit.clamp_overworld_focus(width, height);
}

fn select_overworld_tile(
    campaign: &Campaign,
    camera: (&Camera, &GlobalTransform),
    cursor: Vec2,
    state: &mut OverworldState,
) {
    if let Ok(ray) = camera.0.viewport_to_world(camera.1, cursor) {
        if let Some(pos) = pick_overworld_tile(campaign, ray) {
            if state.selected_tile == Some(pos) {
                if let Some(path) = campaign.world.path(0, pos) {
                    state.path = Some(path);
                    state.moving = true;
                    state.selected_tile = None;
                    state.dirty = true;
                }
            } else {
                state.selected_tile = Some(pos);
                state.dirty = true;
            }
        } else {
            state.selected_tile = None;
            state.dirty = true;
        }
    }
}

fn handle_overworld_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut title_menu: ResMut<TitleMenu>,
    mut next_mode: ResMut<NextState<Mode>>,
    campaign: Option<ResMut<Campaign>>,
    mut state: ResMut<OverworldState>,

    end_q: Query<&Interaction, (Changed<Interaction>, With<EndTurnButton>)>,
    attack_q: Query<&Interaction, (Changed<Interaction>, With<AttackButton>)>,
    over_q: Query<&Interaction, (Changed<Interaction>, With<CampaignOverButton>)>,
    back_q: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut game_state: ResMut<GameState>,
) {
    for int in &over_q {
        if *int == Interaction::Pressed {
            title_menu.open = true;
            next_mode.set(Mode::Classic); // Return to classic title menu mode
            state.campaign_over = false;
            state.campaign_won = false;
            state.encounter = None;
            return;
        }
    }

    // The Main Menu button above is the only active control after the run ends.
    if state.campaign_over {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        title_menu.open = true;
    }

    let Some(mut c) = campaign else { return };

    for int in &end_q {
        if *int == Interaction::Pressed {
            c.world.end_turn();
            let ai_id = c.world.turn_order[c.world.current];
            let encounter = c.world.ai_turn(ai_id);
            state.encounter = encounter;
            if state.encounter.is_none() {
                c.world.end_turn();
            }
            crate::save::store_campaign(&c.world);
            state.dirty = true;
            return;
        }
    }

    for int in &back_q {
        if *int == Interaction::Pressed {
            if matches!(state.encounter, Some(Encounter::Hero(_))) {
                continue;
            }
            let hero = c.world.hero_mut(0).unwrap();
            hero.pos = hero.prev_pos;
            state.encounter = None;
            state.dirty = true;
            return;
        }
    }

    for int in &attack_q {
        if *int == Interaction::Pressed {
            if let Some(enc) = &state.encounter {
                let setup = c.world.battle_setup(0, enc);
                *game_state = GameState::from_setup(&setup);
                next_mode.set(Mode::Deploy);
                return;
            }
        }
    }

    if state.moving || state.encounter.is_some() {
        return;
    } // blocked

    if keys.just_pressed(KeyCode::F6) {
        // Dev keys: F6 = teleport to nearest uncleared camp; F7 = start a rival encounter
        let hero = &c.world.heroes[0];
        let mut nearest = None;
        let mut min_dist = u16::MAX;
        for obj in &c.world.map.objects {
            if let ObjectKind::Camp(_) = obj.kind {
                if !obj.cleared {
                    let d = hero.pos.manhattan(obj.pos);
                    if d < min_dist {
                        min_dist = d;
                        nearest = Some(obj.pos);
                    }
                }
            }
        }
        if let Some(pos) = nearest {
            // Find neighbor to teleport to
            c.world.hero_mut(0).unwrap().pos = MapPos::new(pos.x.saturating_sub(1), pos.y);
            let encounter = c.world.move_hero(0, pos);
            state.encounter = encounter;
            state.dirty = true;
        }
    }

    if keys.just_pressed(KeyCode::F7) {
        if let Some(rival) = c.world.heroes.iter().find(|hero| hero.is_ai && hero.alive) {
            let rival_pos = rival.pos;
            let adjacent = [
                rival_pos.x.checked_sub(1).map(|x| MapPos::new(x, rival_pos.y)),
                rival_pos.x.checked_add(1).map(|x| MapPos::new(x, rival_pos.y)),
                rival_pos.y.checked_sub(1).map(|y| MapPos::new(rival_pos.x, y)),
                rival_pos.y.checked_add(1).map(|y| MapPos::new(rival_pos.x, y)),
            ]
            .into_iter()
            .flatten()
            .find(|pos| tc_world::path::tile_cost(&c.world.map, *pos).is_some());

            if let Some(pos) = adjacent {
                c.world.hero_mut(0).unwrap().pos = pos;
                state.encounter = c.world.move_hero(0, rival_pos);
                state.dirty = true;
            }
        }
    }
}

/// Handles a reward card pick from a campaign battle's post-win draft.
///
/// This is deliberately independent of `Mode`: the draft (and this handler)
/// runs while still in `Mode::OverworldBattle`, so it can't be gated behind
/// `run_if(in_state(Mode::Overworld))` like most overworld input. It must
/// never touch the classic floor-run `RunState` (no `apply_pick` /
/// `record_result` / draft advance) — only the campaign `World`.
fn handle_reward_pick(
    mut pick_events: MessageReader<crate::run::PickCard>,
    campaign: Option<ResMut<Campaign>>,
    mut state: ResMut<OverworldState>,
    mut run: ResMut<crate::run::Run>,
    mut next_mode: ResMut<NextState<Mode>>,
) {
    let Some(mut c) = campaign else { return };

    for pick in pick_events.read() {
        if state.reward_draft.is_none() {
            continue;
        }
        if let Some(item) = tc_run::item::find_item(&pick.0)
            && let tc_run::item::ItemKind::Spell { spell, .. } = item.kind
        {
            c.world.heroes[0].cards.push(spell);
        }
        state.reward_draft = None;
        run.phase = crate::run::RunPhase::Playing;
        crate::save::store_campaign(&c.world);
        state.dirty = true;
        next_mode.set(Mode::Overworld);
    }
}

fn animate_hero(
    mut state: ResMut<OverworldState>,
    mut campaign: Option<ResMut<Campaign>>,
    time: Res<Time>,
    mut next_mode: ResMut<NextState<Mode>>,
) {
    if !state.moving {
        return;
    }
    let Some(mut c) = campaign else {
        return;
    };

    state.anim_timer.tick(time.delta());
    if state.anim_timer.just_finished() {
        if let Some(path) = &mut state.path {
            if !path.is_empty() {
                let next_pos = path.remove(0);
                let cur_pos = c.world.heroes[0].pos;
                if next_pos.x != cur_pos.x {
                    state.facing_left = next_pos.x < cur_pos.x;
                }
                let encounter = c.world.move_hero(0, next_pos);
                state.dirty = true;
                if let Some(enc) = encounter {
                    state.encounter = Some(enc);
                    state.moving = false;
                    state.path = None;
                }
            } else {
                state.moving = false;
                state.path = None;
            }
        }
    }
}

#[derive(Component)]
struct EndTurnButton;

#[derive(Component)]
struct AttackButton;

#[derive(Component)]
struct BackButton;

fn update_overworld_hud(
    mut commands: Commands,
    mut campaign: Option<ResMut<Campaign>>,
    mut state: ResMut<OverworldState>,
    mode: Res<State<Mode>>,
    atlas: Res<Atlas>,
    root: Query<Entity, With<OverworldHudRoot>>,
) {
    if !state.dirty {
        return;
    }
    // Cleared here rather than in `update_overworld_map` (which runs first in
    // the same `Update` tuple and shares this flag) so both systems still see
    // `dirty == true` on the frame that set it.
    state.dirty = false;

    for e in &root {
        commands.entity(e).despawn();
    }

    let Some(c) = campaign else { return };
    let hero = c.world.heroes.first().unwrap();

    let root_ent = commands
        .spawn((
            OverworldHudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .id();

    commands.entity(root_ent).with_children(|parent| {
        // Top bar
        parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    right: Val::Px(80.0),
                    height: Val::Px(40.0),
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ImageNode {
                    image: atlas.image.clone(),
                    rect: Some(atlas.rect("panel_gui_wood")),
                    image_mode: crate::hud::panel_slicer(),
                    ..default()
                },
            ))
            .with_children(|bar| {
                bar.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|left| {
                    left.spawn((
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect("ow_portrait_frame")),
                            ..default()
                        },
                        Node { width: Val::Px(28.0), height: Val::Px(28.0), ..default() },
                    ))
                    .with_children(|frame| {
                        frame.spawn((
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("ow_portrait_sun")),
                                ..default()
                            },
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(3.0),
                                top: Val::Px(3.0),
                                width: Val::Px(22.0),
                                height: Val::Px(22.0),
                                ..default()
                            },
                        ));
                    });
                    left.spawn((
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect("ow_day_frame")),
                            ..default()
                        },
                        Node { width: Val::Px(22.0), height: Val::Px(22.0), ..default() },
                    ));
                    left.spawn((
                        Text::new(format!("Day {} - {}", c.world.day, hero.name)),
                        TextFont { font_size: FontSize::Px(24.0), ..default() },
                        TextColor(crate::hud::INK_WOOD),
                    ));
                });
                bar.spawn((
                    Text::new(format!(
                        "Moves: {:.1}/{:.1}",
                        hero.movement, c.world.params.hero_base_movement
                    )),
                    TextFont { font_size: FontSize::Px(24.0), ..default() },
                    TextColor(crate::hud::INK_WOOD),
                ));
            });

        if *mode.get() == Mode::Overworld {
            // Army strip
            parent
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(50.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(5.0),
                    ..default()
                })
                .with_children(|strip| {
                    for piece in &hero.roster {
                        let icon_name = match piece {
                            tc_core::piece::PieceKind::Pawn => "white_pawn",
                            tc_core::piece::PieceKind::Knight => "white_knight",
                            tc_core::piece::PieceKind::Bishop => "white_bishop",
                            tc_core::piece::PieceKind::Rook => "white_rook",
                            tc_core::piece::PieceKind::Queen => "white_queen",
                            tc_core::piece::PieceKind::King => "white_king",
                            _ => "white_pawn",
                        };
                        strip
                            .spawn((
                                ImageNode {
                                    image: atlas.image.clone(),
                                    rect: Some(atlas.rect("ow_army_slot")),
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(28.0),
                                    height: Val::Px(28.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    ImageNode {
                                        image: atlas.image.clone(),
                                        rect: Some(atlas.rect(icon_name)),
                                        ..default()
                                    },
                                    Node { width: Val::Px(22.0), height: Val::Px(22.0), ..default() },
                                ));
                            });
                    }
                });

            // End turn button
            parent
                .spawn((
                    Button,
                    Interaction::default(),
                    EndTurnButton,
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(20.0),
                        bottom: Val::Px(20.0),
                        width: Val::Px(130.0),
                        height: Val::Px(40.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("btn_wood")),
                        image_mode: crate::hud::button_slicer(),
                        ..default()
                    },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect("ow_end_turn")),
                            ..default()
                        },
                        Node { width: Val::Px(20.0), height: Val::Px(20.0), ..default() },
                    ));
                    btn.spawn((
                        Text::new("End turn"),
                        TextFont { font_size: FontSize::Px(22.0), ..default() },
                        TextColor(crate::hud::INK_WOOD),
                    ));
                });

            if !state.campaign_over
                && let Some(encounter) = &state.encounter
            {
                let (title, details, can_back) = match encounter {
                    Encounter::Camp(obj) => {
                        let (camp_name, defenders) = match &obj.kind {
                            ObjectKind::Camp(CampKind::Village) => ("Village", "Captain, 3 Pawns"),
                            ObjectKind::Camp(CampKind::KnightCamp) => {
                                ("Knight Camp", "Captain, Knight, 3 Pawns")
                            }
                            ObjectKind::Camp(CampKind::BishopCamp) => {
                                ("Bishop Camp", "Captain, Bishop, 3 Pawns")
                            }
                            ObjectKind::Camp(CampKind::Fortress) => ("Fortress", "Captain, Rook, 3 Pawns"),
                            ObjectKind::Camp(CampKind::Citadel) => ("Citadel", "Captain, Queen, 3 Pawns"),
                            _ => ("Camp", "Unknown"),
                        };
                        (format!("Attack {camp_name}?"), format!("Defenders: {defenders}"), true)
                    }
                    Encounter::Hero(rival_id) => {
                        let rival = c.world.hero(*rival_id).map_or("Rival", |hero| hero.name.as_str());
                        (
                            format!("Challenge {rival}?"),
                            "Defeat the rival to claim Oakhaven.".to_string(),
                            false,
                        )
                    }
                };
                parent
                    .spawn(Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Percent(30.0),
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_children(|wrapper| {
                        wrapper
                            .spawn((
                                Node {
                                    width: Val::Percent(90.0),
                                    max_width: Val::Px(380.0),
                                    padding: UiRect::all(Val::Px(16.0)),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    row_gap: Val::Px(8.0),
                                    ..default()
                                },
                                ImageNode {
                                    image: atlas.image.clone(),
                                    rect: Some(atlas.rect("panel_gui_stone")),
                                    image_mode: crate::hud::panel_slicer(),
                                    ..default()
                                },
                            ))
                            .with_children(|panel| {
                                panel.spawn((
                                    Text::new(&title),
                                    TextFont { font_size: FontSize::Px(28.0), ..default() },
                                    TextColor(crate::hud::INK_WOOD),
                                    TextLayout { justify: Justify::Center, ..default() },
                                ));

                                panel.spawn((
                                    Text::new(details),
                                    TextFont { font_size: FontSize::Px(18.0), ..default() },
                                    TextColor(crate::hud::INK_WOOD),
                                    TextLayout { justify: Justify::Center, ..default() },
                                ));

                                panel
                                    .spawn(Node {
                                        flex_direction: FlexDirection::Row,
                                        margin: UiRect::top(Val::Px(8.0)),
                                        column_gap: Val::Px(12.0),
                                        ..default()
                                    })
                                    .with_children(|row| {
                                        row.spawn((
                                            Button,
                                            Interaction::default(),
                                            AttackButton,
                                            Node {
                                                width: Val::Px(112.0),
                                                height: Val::Px(40.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            ImageNode {
                                                image: atlas.image.clone(),
                                                rect: Some(atlas.rect("btn_gold")),
                                                image_mode: crate::hud::button_slicer(),
                                                ..default()
                                            },
                                        ))
                                        .with_child((
                                            Text::new("Fight"),
                                            TextFont { font_size: FontSize::Px(24.0), ..default() },
                                            TextColor(crate::hud::INK_WOOD),
                                        ));

                                        if can_back {
                                            row.spawn((
                                                Button,
                                                Interaction::default(),
                                                BackButton,
                                                Node {
                                                    width: Val::Px(100.0),
                                                    height: Val::Px(40.0),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    ..default()
                                                },
                                                ImageNode {
                                                    image: atlas.image.clone(),
                                                    rect: Some(atlas.rect("btn_wood")),
                                                    image_mode: crate::hud::button_slicer(),
                                                    ..default()
                                                },
                                            ))
                                            .with_child((
                                                Text::new("Back"),
                                                TextFont { font_size: FontSize::Px(24.0), ..default() },
                                                TextColor(crate::hud::INK_WOOD),
                                            ));
                                        }
                                    });
                            });
                    });
            }
        }
        if state.campaign_over {
            parent
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Percent(35.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|wrapper| {
                    wrapper
                        .spawn((
                            Node {
                                width: Val::Percent(90.0),
                                max_width: Val::Px(380.0),
                                padding: UiRect::all(Val::Px(16.0)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: Val::Px(8.0),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("panel_gui_stone")),
                                image_mode: crate::hud::panel_slicer(),
                                ..default()
                            },
                        ))
                        .with_children(|panel| {
                            panel.spawn((
                                Text::new(if state.campaign_won {
                                    "Victory — Oakhaven is yours"
                                } else {
                                    "Defeat"
                                }),
                                TextFont { font_size: FontSize::Px(28.0), ..default() },
                                TextColor(crate::hud::INK_WOOD),
                                TextLayout { justify: Justify::Center, ..default() },
                            ));
                            panel.spawn((
                                Text::new(format!("Days taken: {}", c.world.day)),
                                TextFont { font_size: FontSize::Px(20.0), ..default() },
                                TextColor(crate::hud::INK_WOOD),
                            ));
                            panel
                                .spawn((
                                    Button,
                                    Interaction::default(),
                                    CampaignOverButton,
                                    Node {
                                        width: Val::Px(160.0),
                                        height: Val::Px(40.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        margin: UiRect::top(Val::Px(20.0)),
                                        ..default()
                                    },
                                    ImageNode {
                                        image: atlas.image.clone(),
                                        rect: Some(atlas.rect("btn_gold")),
                                        image_mode: crate::hud::button_slicer(),
                                        ..default()
                                    },
                                ))
                                .with_child((
                                    Text::new("Main Menu"),
                                    TextFont { font_size: FontSize::Px(24.0), ..default() },
                                    TextColor(crate::hud::INK_WOOD),
                                ));
                        });
                });
        }
    });
}

#[derive(Component)]
struct DeployRoot;

#[derive(Component)]
struct StartBattleButton;

#[derive(Component)]
struct AutoDeployButton;

#[derive(Component)]
struct DeckButton;

#[derive(Component)]
struct DeckPanelRoot;

#[derive(Component)]
struct DeckCloseButton;

#[derive(Component)]
struct DeckCollectionList;

#[derive(Component)]
struct DeckCardButton(usize);

#[derive(Component)]
struct DeckCardLabel(usize);

#[derive(Component)]
struct DeckCountText;

#[derive(Component)]
struct DraftCardButton(String);

#[derive(Component)]
struct CampaignOverButton;

#[derive(Component)]
struct RetreatButton;
#[derive(Component)]
struct BattleHudRoot;

#[derive(Component)]
struct DeployZoneMarker;

fn setup_deploy(
    mut commands: Commands,
    atlas: Res<Atlas>,
    game_state: Res<GameState>,
    campaign: Option<Res<Campaign>>,
    state: Res<OverworldState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Highlight the player's deploy zone (the two ranks nearest their side; see
    // `handle_deploy_input`'s `sq.y <= 1` check) with the blue deploy tile.
    let mut q = Quads::default();
    let uv = full_uv(atlas.uv("ow_deploy_blue"));
    for y in 0..game_state.size.min(2) {
        for x in 0..game_state.size {
            let sq = tc_core::Sq::new(x, y);
            let top = square_top(sq, game_state.game.terrain.height(sq)) + Vec3::Y * 0.01;
            q.add(flat(top, Vec2::splat(0.9)), uv, 1.0);
        }
    }
    if !q.pos.is_empty() {
        let mut mat = StandardMaterial::from(Color::WHITE);
        mat.base_color_texture = Some(atlas.image.clone());
        mat.alpha_mode = AlphaMode::Blend;
        mat.unlit = true;
        commands.spawn((DeployZoneMarker, Mesh3d(meshes.add(q.mesh())), MeshMaterial3d(materials.add(mat))));
    }

    let cards = campaign.as_ref().map_or(&[][..], |c| c.world.heroes[0].cards.as_slice());
    commands
        .spawn((
            DeployRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                bottom: Val::Px(12.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|p| {
                p.spawn((
                    Button,
                    Interaction::default(),
                    AutoDeployButton,
                    Node {
                        width: Val::Px(82.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("btn_gold")),
                        image_mode: crate::hud::button_slicer(),
                        ..default()
                    },
                ))
                .with_child((
                    Text::new("Auto"),
                    TextFont { font_size: FontSize::Px(20.0), ..default() },
                    TextColor(crate::hud::INK_WOOD),
                ));

                p.spawn((
                    Button,
                    Interaction::default(),
                    DeckButton,
                    Node {
                        width: Val::Px(82.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("btn_wood")),
                        image_mode: crate::hud::button_slicer(),
                        ..default()
                    },
                ))
                .with_child((
                    Text::new("Deck"),
                    TextFont { font_size: FontSize::Px(20.0), ..default() },
                    TextColor(crate::hud::INK_WOOD),
                ));

                p.spawn((
                    Button,
                    Interaction::default(),
                    StartBattleButton,
                    ButtonDisabled(false),
                    ButtonVisuals::GOLD,
                    Node {
                        width: Val::Px(142.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("btn_gold")),
                        image_mode: crate::hud::button_slicer(),
                        ..default()
                    },
                ))
                .with_child((
                    Text::new("Start battle"),
                    TextFont { font_size: FontSize::Px(20.0), ..default() },
                    TextColor(crate::hud::INK_WOOD),
                ));
            });

            root.spawn((
                DeckPanelRoot,
                GlobalZIndex(100),
                Button,
                Interaction::default(),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    display: if state.deck_open { Display::Flex } else { Display::None },
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
            ))
            .with_children(|modal| {
                modal
                    .spawn((
                        Node {
                            width: Val::Percent(90.0),
                            max_width: Val::Px(460.0),
                            height: Val::Percent(72.0),
                            max_height: Val::Px(620.0),
                            min_height: Val::Px(300.0),
                            padding: UiRect::all(Val::Px(18.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(10.0),
                            ..default()
                        },
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect("panel_gui_stone")),
                            image_mode: panel_slicer(),
                            ..default()
                        },
                    ))
                    .with_children(|panel| {
                        panel
                            .spawn(Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(8.0)),
                                ..default()
                            })
                            .with_children(|header| {
                                header.spawn((
                                    DeckCountText,
                                    Text::new("Deck 0/15"),
                                    TextFont { font_size: FontSize::Px(24.0), ..default() },
                                    TextColor(crate::hud::INK_WOOD),
                                ));
                                header
                                    .spawn((
                                        Button,
                                        Interaction::default(),
                                        DeckCloseButton,
                                        Node {
                                            width: Val::Px(86.0),
                                            height: Val::Px(38.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        ImageNode {
                                            image: atlas.image.clone(),
                                            rect: Some(atlas.rect("btn_wood")),
                                            image_mode: crate::hud::button_slicer(),
                                            ..default()
                                        },
                                    ))
                                    .with_child((
                                        Text::new("Done"),
                                        TextFont { font_size: FontSize::Px(20.0), ..default() },
                                        TextColor(crate::hud::INK_WOOD),
                                    ));
                            });

                        panel
                            .spawn((
                                DeckCollectionList,
                                ScrollPosition::default(),
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_grow: 1.0,
                                    min_height: Val::Px(0.0),
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    align_content: AlignContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::SpaceBetween,
                                    row_gap: Val::Px(8.0),
                                    column_gap: Val::Px(8.0),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                            ))
                            .with_children(|list| {
                                for (index, spell) in cards.iter().copied().enumerate() {
                                    list.spawn((
                                        Button,
                                        Interaction::default(),
                                        DeckCardButton(index),
                                        Node {
                                            width: Val::Percent(48.0),
                                            min_height: Val::Px(48.0),
                                            padding: UiRect::horizontal(Val::Px(5.0)),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        ImageNode {
                                            image: atlas.image.clone(),
                                            rect: Some(atlas.rect("btn_wood")),
                                            image_mode: crate::hud::button_slicer(),
                                            ..default()
                                        },
                                    ))
                                    .with_child((
                                        DeckCardLabel(index),
                                        Text::new(spell_label(spell)),
                                        TextFont { font_size: FontSize::Px(16.0), ..default() },
                                        TextColor(crate::hud::INK_WOOD),
                                        TextLayout { justify: Justify::Center, ..default() },
                                    ));
                                }
                            });
                    });
            });
        });
}

fn teardown_deploy(
    mut commands: Commands,
    root: Query<Entity, With<DeployRoot>>,
    zone: Query<Entity, With<DeployZoneMarker>>,
    mut state: ResMut<OverworldState>,
) {
    state.deck_open = false;
    for e in &root {
        commands.entity(e).despawn();
    }
    for e in &zone {
        commands.entity(e).despawn();
    }
}

fn handle_deploy(
    start_q: Query<(&Interaction, &ButtonDisabled), (Changed<Interaction>, With<StartBattleButton>)>,
    auto_q: Query<&Interaction, (Changed<Interaction>, With<AutoDeployButton>)>,
    mut next_mode: ResMut<NextState<Mode>>,
    mut game_state: ResMut<GameState>,
    mut campaign: Option<ResMut<Campaign>>,
    state: Res<OverworldState>,
) {
    for (int, disabled) in &start_q {
        if *int == Interaction::Pressed && !disabled.0 {
            if let Some(campaign) = campaign.as_ref() {
                let hero = &campaign.world.heroes[0];
                let deck = if hero.has_valid_deck() {
                    hero.deck.clone()
                } else {
                    hero.cards.iter().take(15).copied().collect()
                };
                game_state.game.set_deck(tc_core::Side::White, deck);
            }
            game_state.selected = None;
            game_state.pieces_dirty = true;
            next_mode.set(Mode::OverworldBattle);
        }
    }
    for int in &auto_q {
        if *int == Interaction::Pressed {
            if let Some(ref mut c) = campaign {
                if let Some(enc) = &state.encounter {
                    let setup = c.world.battle_setup(0, enc);
                    *game_state = GameState::from_setup(&setup);
                }
            }
        }
    }
}

fn handle_deck_interactions(
    deck_q: Query<&Interaction, (Changed<Interaction>, With<DeckButton>)>,
    close_q: Query<&Interaction, (Changed<Interaction>, With<DeckCloseButton>)>,
    card_q: Query<(&Interaction, &DeckCardButton), (Changed<Interaction>, With<Button>)>,
    mut campaign: Option<ResMut<Campaign>>,
    mut state: ResMut<OverworldState>,
) {
    for interaction in &deck_q {
        if *interaction == Interaction::Pressed {
            state.deck_open = !state.deck_open;
        }
    }
    for interaction in &close_q {
        if *interaction == Interaction::Pressed {
            state.deck_open = false;
        }
    }

    let Some(ref mut campaign) = campaign else { return };
    for (interaction, card) in &card_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let hero = &mut campaign.world.heroes[0];
        let Some(&spell) = hero.cards.get(card.0) else { continue };
        if let Some(selected) = hero.deck.iter().position(|chosen| *chosen == spell) {
            hero.deck.remove(selected);
            crate::save::store_campaign(&campaign.world);
        } else if hero.deck.len() < 15 {
            hero.deck.push(spell);
            crate::save::store_campaign(&campaign.world);
        }
    }
}

fn update_deck_ui(
    campaign: Option<Res<Campaign>>,
    state: Res<OverworldState>,
    atlas: Res<Atlas>,
    mut panel_q: Query<&mut Node, With<DeckPanelRoot>>,
    mut count_q: Query<&mut Text, With<DeckCountText>>,
    mut label_q: Query<(&mut Text, &DeckCardLabel), Without<DeckCountText>>,
    mut card_q: Query<(&DeckCardButton, &mut ImageNode), Without<ButtonVisuals>>,
) {
    for mut node in &mut panel_q {
        node.display = if state.deck_open { Display::Flex } else { Display::None };
    }
    let Some(campaign) = campaign else { return };
    let hero = &campaign.world.heroes[0];
    let selected_count = hero.deck.len();
    for mut text in &mut count_q {
        text.0 = format!("Deck {selected_count}/15");
    }
    for (mut text, label) in &mut label_q {
        let Some(&spell) = hero.cards.get(label.0) else { continue };
        text.0 = spell_label(spell);
    }
    for (card, mut image) in &mut card_q {
        let Some(&spell) = hero.cards.get(card.0) else { continue };
        let occurrence = hero.cards[..card.0].iter().filter(|&&card| card == spell).count();
        let selected = hero.deck.iter().filter(|&&card| card == spell).count() > occurrence;
        let rect = atlas.rect(if selected { "btn_gold" } else { "btn_wood" });
        if image.rect != Some(rect) {
            image.rect = Some(rect);
        }
    }
}

fn update_deploy_start(
    campaign: Option<Res<Campaign>>,
    mut start_q: Query<&mut ButtonDisabled, With<StartBattleButton>>,
) {
    let allowed = campaign.is_some_and(|campaign| {
        let hero = &campaign.world.heroes[0];
        let required = hero.cards.len().min(15);
        hero.deck.len() == required
    });
    for mut disabled in &mut start_q {
        disabled.0 = !allowed;
    }
}

fn scroll_deck_list(
    mut wheel: MessageReader<MouseWheel>,
    state: Res<OverworldState>,
    mut list_q: Query<(&mut ScrollPosition, &ComputedNode), With<DeckCollectionList>>,
) {
    for event in wheel.read() {
        if !state.deck_open {
            continue;
        }
        let amount = match event.unit {
            MouseScrollUnit::Line => event.y * 42.0,
            MouseScrollUnit::Pixel => event.y,
        };
        for (mut scroll, computed) in &mut list_q {
            let max_y = (computed.content_size().y - computed.size().y) * computed.inverse_scale_factor();
            scroll.y = (scroll.y - amount).clamp(0.0, max_y.max(0.0));
        }
    }
}

fn spell_label(spell: tc_core::SpellId) -> String {
    let name = match spell {
        tc_core::SpellId::RaiseEarth => "Raise Earth",
        tc_core::SpellId::LowerEarth => "Lower Earth",
        tc_core::SpellId::Freeze => "Freeze",
        tc_core::SpellId::Bridge => "Bridge",
        tc_core::SpellId::DigTunnel => "Dig Tunnel",
        tc_core::SpellId::Shield => "Shield",
        tc_core::SpellId::Swap => "Swap",
        tc_core::SpellId::Rewind => "Rewind",
    };
    let kind = if spell.is_quick() { "Quick" } else { "Action" };
    format!("{name} · {kind}")
}

fn handle_deploy_input(
    mut game_state: ResMut<GameState>,
    state: Res<OverworldState>,
    mouse: Res<ButtonInput<MouseButton>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    atlas: Res<Atlas>,
) {
    if state.deck_open {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let (cam, gtf) = *camera;
    let ground = || {
        cam.viewport_to_world(gtf, cursor)
            .ok()
            .and_then(|ray| crate::board_view::pick_square(&game_state, ray))
    };
    let piece = crate::board_view::pick_piece(&game_state, &atlas, *camera, cursor);

    let Some(sq) = piece.or_else(ground) else { return };

    let clicked_piece = game_state.game.pos.get(sq);
    let is_player = clicked_piece.is_some_and(|p| p.side == tc_core::piece::Side::White);

    if let Some(sel) = game_state.selected {
        if sel == sq {
            game_state.selected = None;
            game_state.pieces_dirty = true;
        } else if is_player {
            let mut pieces = vec![];
            for y in 0..game_state.size {
                for x in 0..game_state.size {
                    let s = tc_core::Sq::new(x, y);
                    if let Some(p) = game_state.game.pos.get(s) {
                        let mut target = s;
                        if s == sel {
                            target = sq;
                        } else if s == sq {
                            target = sel;
                        }
                        pieces.push((target, p));
                    }
                }
            }
            if let Ok(new_pos) = tc_core::position::Position::from_placement(game_state.size, &pieces) {
                game_state.game.pos = new_pos;
                game_state.selected = None;
                game_state.pieces_dirty = true;
            }
        } else if clicked_piece.is_none() && sq.y <= 1 {
            let mut pieces = vec![];
            for y in 0..game_state.size {
                for x in 0..game_state.size {
                    let s = tc_core::Sq::new(x, y);
                    if let Some(p) = game_state.game.pos.get(s) {
                        let target = if s == sel { sq } else { s };
                        pieces.push((target, p));
                    }
                }
            }
            if let Ok(new_pos) = tc_core::position::Position::from_placement(game_state.size, &pieces) {
                game_state.game.pos = new_pos;
                game_state.selected = None;
                game_state.pieces_dirty = true;
            }
        } else {
            game_state.selected = None;
            game_state.pieces_dirty = true;
        }
    } else if is_player {
        game_state.selected = Some(sq);
        game_state.pieces_dirty = true;
    }
}

fn handle_battle(
    keys: Res<ButtonInput<KeyCode>>,
    retreat_q: Query<&Interaction, (Changed<Interaction>, With<RetreatButton>)>,
    mut game_state: ResMut<GameState>,
    mut campaign: Option<ResMut<Campaign>>,
    mut state: ResMut<OverworldState>,
    mut next_mode: ResMut<NextState<Mode>>,
    mut run: ResMut<crate::run::Run>,
) {
    let Some(mut c) = campaign else { return };

    let mut do_outcome = |outcome: tc_world::encounter::Outcome,
                          c: &mut Campaign,
                          game_state: &mut GameState,
                          state: &mut OverworldState| {
        let init = if let Some(first) = game_state.undo.first() { first } else { &game_state.game };
        let mut lost = vec![];
        let mut enemy_lost = vec![];

        let mut my_init = 0;
        let mut en_init = 0;
        let mut my_end = 0;
        let mut en_end = 0;

        // Count pieces (simple logic, just extracting lost pieces from lists could be tricky,
        // but we just assume all pieces are tracked).
        // For accurate tracking, we'd compare initial list vs final list.
        for y in 0..game_state.size {
            for x in 0..game_state.size {
                let s = tc_core::Sq::new(x, y);
                if let Some(p) = init.pos.get(s) {
                    if p.side == tc_core::piece::Side::White {
                        my_init += 1;
                    } else {
                        en_init += 1;
                    }
                }
                if let Some(p) = game_state.game.pos.get(s) {
                    if p.side == tc_core::piece::Side::White {
                        my_end += 1;
                    } else {
                        en_end += 1;
                    }
                }
            }
        }
        // Since we don't know EXACTLY which kind was lost easily, let's just use pawns for dummy tracking or diff the arrays.
        // Actually, tc_core has a way to iterate pieces!
        let mut init_white = vec![];
        let mut init_black = vec![];
        let mut end_white = vec![];
        let mut end_black = vec![];
        for y in 0..game_state.size {
            for x in 0..game_state.size {
                let s = tc_core::Sq::new(x, y);
                if let Some(p) = init.pos.get(s) {
                    if p.side == tc_core::piece::Side::White {
                        init_white.push(p.kind);
                    } else {
                        init_black.push(p.kind);
                    }
                }
                if let Some(p) = game_state.game.pos.get(s) {
                    if p.side == tc_core::piece::Side::White {
                        end_white.push(p.kind);
                    } else {
                        end_black.push(p.kind);
                    }
                }
            }
        }
        for k in end_white {
            if let Some(idx) = init_white.iter().position(|&x| x == k) {
                init_white.remove(idx);
            }
        }
        for k in end_black {
            if let Some(idx) = init_black.iter().position(|&x| x == k) {
                init_black.remove(idx);
            }
        }
        lost = init_white;
        enemy_lost = init_black;

        let encounter = state.encounter.take();
        let hero_battle = matches!(encounter, Some(Encounter::Hero(_)));
        if let Some(enc) = encounter {
            c.world.apply_battle(0, enc, tc_world::encounter::BattleResult { outcome, lost, enemy_lost });
        }
        state.dirty = true;

        let player_alive = c.world.hero(0).is_some_and(|hero| hero.alive);
        if hero_battle || !player_alive {
            state.campaign_won = hero_battle && c.world.winner() == Some(0);
            state.campaign_over = true;
            state.reward_draft = None;
            state.encounter = None;
            run.phase = crate::run::RunPhase::Playing;
            crate::save::clear_campaign();
            next_mode.set(Mode::Overworld);
            return;
        }

        if outcome == tc_world::encounter::Outcome::Won {
            crate::save::store_campaign(&c.world);
            let catalog = tc_run::item::catalog();
            let mut spells: Vec<_> =
                catalog.iter().filter(|i| matches!(i.kind, tc_run::item::ItemKind::Spell { .. })).collect();
            let mut chosen = ["".to_string(), "".to_string(), "".to_string()];

            let mut r = c.world.next_rng();
            // Sample without replacement so the three offers are always distinct spells.
            for slot in &mut chosen {
                if spells.is_empty() {
                    break;
                }
                let idx = (r.next_u64() as usize) % spells.len();
                *slot = spells.swap_remove(idx).id.clone();
            }
            run.phase = crate::run::RunPhase::Draft(chosen.clone());
            state.reward_draft = Some(chosen);
            // Stay in OverworldBattle so the Draft UI displays
        } else if outcome == tc_world::encounter::Outcome::Retreated {
            crate::save::store_campaign(&c.world);
            next_mode.set(Mode::Overworld);
        } else {
            state.campaign_won = false;
            state.campaign_over = true;
            crate::save::clear_campaign();
            state.encounter = None;
            state.reward_draft = None;
            next_mode.set(Mode::Overworld);
        }
    };

    if let Some(outcome) = game_state.outcome {
        let result_outcome = match outcome {
            tc_core::Outcome::Checkmate { winner: tc_core::Side::White } => tc_world::encounter::Outcome::Won,
            _ => tc_world::encounter::Outcome::Checkmated,
        };
        do_outcome(result_outcome, &mut c, &mut game_state, &mut state);
        game_state.outcome = None;
        return;
    }

    if keys.just_pressed(KeyCode::F8) {
        do_outcome(tc_world::encounter::Outcome::Won, &mut c, &mut game_state, &mut state);
        return;
    }

    for int in &retreat_q {
        if *int == Interaction::Pressed && !matches!(state.encounter, Some(Encounter::Hero(_))) {
            do_outcome(tc_world::encounter::Outcome::Retreated, &mut c, &mut game_state, &mut state);
            return;
        }
    }

    if keys.just_pressed(KeyCode::KeyR) && !matches!(state.encounter, Some(Encounter::Hero(_))) {
        do_outcome(tc_world::encounter::Outcome::Retreated, &mut c, &mut game_state, &mut state);
        return;
    }
}
