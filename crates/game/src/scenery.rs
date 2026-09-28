//! Island surroundings for the 3D board: island ring, calm sea, props, sky dome, and fog.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use tc_core::Sq;

use crate::atlas::Atlas;
use crate::board_view::{
    Billboard, Look, PX, Quads, board_center, card_mesh, flat, full_uv, hash, rotate_uv, top_y, wall,
};
use crate::game::GameState;
use crate::input::MainCamera;

/// Sea surface level.
pub(crate) const SEA_Y: f32 = -0.55;

/// Tag for entities rebuilt when the board seed or size changes.
#[derive(Component)]
pub(crate) struct SceneryPart;

/// Marker for the sky dome sphere and horizon cylinders centred on the board.
#[derive(Component)]
struct SkyDome;

/// One of the two sea frames (shallow, deep, and shore); the other is hidden.
#[derive(Component)]
struct SeaFrame(usize);

/// Cloud billboard that drifts around the board center.
#[derive(Component)]
struct CloudCard {
    initial_angle: f32,
    radius: f32,
    height: f32,
}

/// Circling bird with flap animation.
#[derive(Component)]
struct Bird {
    radius: f32,
    height: f32,
    angular_speed: f32,
    start_angle: f32,
    flap_period: f32,
    frames: [Handle<Mesh>; 2],
}

#[derive(Clone, Copy, PartialEq)]
enum CellType {
    Board(i8),
    Shore,
    Grass(i8),
    Sea,
}

fn haze_color(factor: f32) -> Color {
    let haze = Color::srgb_u8(0xA9, 0xB8, 0xC4).to_linear();
    let r = 1.0 + (haze.red - 1.0) * factor;
    let g = 1.0 + (haze.green - 1.0) * factor;
    let b = 1.0 + (haze.blue - 1.0) * factor;
    Color::linear_rgb(r, g, b)
}

fn build_horizon_cylinder(atlas: &Atlas, name: &str, radius: f32, bottom_y: f32, cylinder_top: f32) -> Mesh {
    let mut q = Quads::default();
    let uv = full_uv(atlas.uv(name));
    let segs = 24;
    let step = 2.0 * std::f32::consts::PI / segs as f32;
    for k in 0..segs {
        let theta_0 = k as f32 * step;
        let theta_1 = (k + 1) as f32 * step;
        let p0 = Vec3::new(radius * theta_0.sin(), 0.0, -radius * theta_0.cos());
        let p1 = Vec3::new(radius * theta_1.sin(), 0.0, -radius * theta_1.cos());
        q.add(
            [
                Vec3::new(p0.x, bottom_y, p0.z),
                Vec3::new(p1.x, bottom_y, p1.z),
                Vec3::new(p1.x, cylinder_top, p1.z),
                Vec3::new(p0.x, cylinder_top, p0.z),
            ],
            uv,
            1.0,
        );
    }
    q.mesh()
}

/// Seeded smooth value noise in 0..1 using corner hashing with bilinear smoothstep.
fn noise(x: f32, y: f32, freq: f32, salt: u32, seed: u64) -> f32 {
    let px = x * freq;
    let py = y * freq;
    let x0 = px.floor() as i32;
    let y0 = py.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let fx = px - px.floor();
    let fy = py - py.floor();
    let wx = fx * fx * (3.0 - 2.0 * fx);
    let wy = fy * fy * (3.0 - 2.0 * fy);

    let corner = |ix: i32, iy: i32| -> f32 {
        let s = (seed as u32) ^ ((seed >> 32) as u32).wrapping_mul(0x85eb_ca6b) ^ salt;
        let h = hash(ix, iy, s);
        (h as f32) / (u32::MAX as f32)
    };

    let v00 = corner(x0, y0);
    let v10 = corner(x1, y0);
    let v01 = corner(x0, y1);
    let v11 = corner(x1, y1);

    let v0 = v00 + wx * (v10 - v00);
    let v1 = v01 + wx * (v11 - v01);
    (v0 + wy * (v1 - v0)).clamp(0.0, 1.0)
}

/// Euclidean distance from an integer cell to the board rectangle `[0, n-1] × [0, n-1]`.
fn rect_distance(x: i32, y: i32, n: i32) -> f32 {
    let max_c = (n - 1) as f32;
    let xf = x as f32;
    let yf = y as f32;
    let dx = if xf < 0.0 {
        -xf
    } else if xf > max_c {
        xf - max_c
    } else {
        0.0
    };
    let dy = if yf < 0.0 {
        -yf
    } else if yf > max_c {
        yf - max_c
    } else {
        0.0
    };
    dx.hypot(dy)
}

fn setup_sky(
    mut commands: Commands,
    state: Res<GameState>,
    atlas: Res<Atlas>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut mesh = Sphere::new(400.0).mesh().uv(48, 24);
    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        let colors: Vec<[f32; 4]> = positions
            .iter()
            .map(|&[_x, y, _z]| {
                let ny = (y / 400.0).clamp(-1.0, 1.0);
                if ny < 0.0 {
                    Color::srgb_u8(0x6F, 0x87, 0x97).to_linear().to_f32_array()
                } else {
                    let s = ny * ny * (3.0 - 2.0 * ny);
                    let h = Color::srgb_u8(0xA9, 0xB8, 0xC4).to_linear();
                    let z = Color::srgb_u8(0x3A, 0x4B, 0x8F).to_linear();
                    let r = h.red + (z.red - h.red) * s;
                    let g = h.green + (z.green - h.green) * s;
                    let b = h.blue + (z.blue - h.blue) * s;
                    [r, g, b, 1.0]
                }
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        fog_enabled: false,
        ..default()
    });
    let center = board_center(state.size);
    commands.spawn((
        SkyDome,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Transform::from_translation(center),
    ));

    for (name, radius, cylinder_top, haze_mix) in
        [("sky_far", 300.0, 17.0, 0.6), ("sky_mid", 250.0, 9.0, 0.4), ("sky_near", 205.0, 4.5, 0.2)]
    {
        let cylinder = build_horizon_cylinder(&atlas, name, radius, SEA_Y, cylinder_top);
        let band_material = materials.add(StandardMaterial {
            base_color: haze_color(haze_mix),
            base_color_texture: Some(atlas.image.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            fog_enabled: false,
            ..default()
        });
        commands.spawn((
            SkyDome,
            Mesh3d(meshes.add(cylinder)),
            MeshMaterial3d(band_material),
            Transform::from_translation(center),
        ));
    }

    let cloud_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.97, 0.97, 0.97),
        base_color_texture: Some(atlas.image.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        fog_enabled: false,
        ..default()
    });

    for i in 0..12 {
        let name =
            ["cloud_0", "cloud_1", "cloud_2", "cloud_3", "cloud_4", "cloud_5", "cloud_6", "cloud_7"][i % 8];
        let size = atlas.px(name) * PX * 9.0;
        let hw = size.x / 2.0;
        let hh = size.y / 2.0;
        let mut q = Quads::default();
        q.add(
            [
                Vec3::new(-hw, -hh, 0.0),
                Vec3::new(hw, -hh, 0.0),
                Vec3::new(hw, hh, 0.0),
                Vec3::new(-hw, hh, 0.0),
            ],
            full_uv(atlas.uv(name)),
            1.0,
        );

        let base_angle = (i as f32 * 30.0).to_radians();
        let jitter = ((hash(i as i32, 50, 500) as f32 / u32::MAX as f32) * 2.0 - 1.0) * 15.0f32.to_radians();
        let initial_angle = base_angle + jitter;
        let radius = 170.0 + 60.0 * (hash(i as i32, 51, 501) as f32 / u32::MAX as f32);
        let height = 30.0 + 25.0 * (hash(i as i32, 52, 502) as f32 / u32::MAX as f32);
        let pos = Vec3::new(
            center.x + radius * initial_angle.cos(),
            height,
            center.z + radius * initial_angle.sin(),
        );

        commands.spawn((
            CloudCard { initial_angle, radius, height },
            Mesh3d(meshes.add(q.mesh())),
            MeshMaterial3d(cloud_material.clone()),
            Transform::from_translation(pos),
        ));
    }
}

fn update_sky_dome(state: Res<GameState>, mut sky: Query<&mut Transform, With<SkyDome>>) {
    let center = board_center(state.size);
    for mut tf in &mut sky {
        tf.translation = center;
    }
}

fn update_clouds(
    time: Res<Time>,
    state: Res<GameState>,
    camera: Single<&Transform, (With<MainCamera>, Without<CloudCard>)>,
    mut clouds: Query<(&CloudCard, &mut Transform), Without<MainCamera>>,
) {
    let back = camera.back();
    let yaw = back.x.atan2(back.z);
    let rotation = Quat::from_rotation_y(yaw);
    let center = board_center(state.size);
    let t = time.elapsed_secs();

    for (cloud, mut tf) in &mut clouds {
        let angle = cloud.initial_angle + t * 0.01;
        tf.translation = Vec3::new(
            center.x + cloud.radius * angle.cos(),
            cloud.height,
            center.z + cloud.radius * angle.sin(),
        );
        tf.rotation = rotation;
    }
}

fn setup_birds(
    mut commands: Commands,
    atlas: Res<Atlas>,
    look: Option<ResMut<Look>>,
    mut meshes: ResMut<Assets<Mesh>>,
    state: Res<GameState>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some(mut look) = look else { return };
    *spawned = true;

    let center = board_center(state.size);
    for i in 0..4 {
        let is_crow = i == 3;
        let radius = 14.0 + 3.0 * i as f32;
        let height = 10.0 + 1.5 * i as f32;
        let flap_period = if is_crow { 0.22 } else { 0.18 };
        let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
        let speed = 0.25 + 0.05 * i as f32;
        let angular_speed = dir * speed;
        let start_angle = i as f32 * (std::f32::consts::PI / 2.0);

        let (name_0, name_1) = if is_crow { ("crow_0", "crow_1") } else { ("gull_0", "gull_1") };
        let mesh_0 = card_mesh(&mut look, &mut meshes, &atlas, name_0, false);
        let mesh_1 = card_mesh(&mut look, &mut meshes, &atlas, name_1, false);

        let pos =
            Vec3::new(center.x + radius * start_angle.cos(), height, center.z + radius * start_angle.sin());

        commands.spawn((
            Bird {
                radius,
                height,
                angular_speed,
                start_angle,
                flap_period,
                frames: [mesh_0.clone(), mesh_1],
            },
            Billboard,
            Mesh3d(mesh_0),
            MeshMaterial3d(look.cards.clone()),
            Transform::from_translation(pos),
        ));
    }
}

fn update_birds(
    time: Res<Time>,
    state: Res<GameState>,
    mut birds: Query<(&Bird, &mut Transform, &mut Mesh3d)>,
) {
    let center = board_center(state.size);
    let t = time.elapsed_secs();
    for (bird, mut tf, mut mesh) in &mut birds {
        let angle = bird.start_angle + bird.angular_speed * t;
        tf.translation = Vec3::new(
            center.x + bird.radius * angle.cos(),
            bird.height,
            center.z + bird.radius * angle.sin(),
        );
        let frame = (t / bird.flap_period) as usize % 2;
        let want_mesh = &bird.frames[frame];
        if mesh.0 != *want_mesh {
            mesh.0 = want_mesh.clone();
        }
    }
}

fn animate_sea(time: Res<Time>, mut q: Query<(&SeaFrame, &mut Visibility)>) {
    let frame = (time.elapsed_secs() / 0.7) as usize % 2;
    for (sea, mut v) in &mut q {
        let want = if sea.0 == frame { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
}

fn update_camera_fog(
    state: Res<GameState>,
    mut camera: Query<(&Transform, &mut DistanceFog), With<MainCamera>>,
) {
    let n = state.size as f32;
    let center = board_center(state.size);
    for (cam_tf, mut fog) in &mut camera {
        let dist = cam_tf.translation.distance(center);
        let start = dist + n;
        let end = start + 160.0 + 2.0 * n;
        fog.falloff = FogFalloff::Linear { start, end };
    }
}

#[allow(clippy::too_many_arguments)] // a Bevy system: one parameter per resource
fn rebuild_scenery(
    mut commands: Commands,
    state: Res<GameState>,
    atlas: Res<Atlas>,
    look: Option<ResMut<Look>>,
    mut meshes: ResMut<Assets<Mesh>>,
    old: Query<Entity, With<SceneryPart>>,
    mut last: Local<Option<(u64, u8)>>,
) {
    let Some(mut look) = look else { return };
    if *last == Some((state.seed, state.size)) {
        return;
    }
    *last = Some((state.seed, state.size));
    for e in &old {
        commands.entity(e).despawn();
    }

    let n = state.size as i32;
    let r = 6 + n / 4;
    let seed = state.seed;
    let min_coord = -r;
    let max_coord = n - 1 + r;
    let span = (max_coord - min_coord + 1) as usize;

    let mut grid = vec![vec![CellType::Sea; span]; span];

    for x in min_coord..=max_coord {
        for y in min_coord..=max_coord {
            let xi = (x - min_coord) as usize;
            let yi = (y - min_coord) as usize;
            if x >= 0 && x < n && y >= 0 && y < n {
                let sq = Sq::new(x as u8, y as u8);
                grid[xi][yi] = CellType::Board(state.game.terrain.height(sq) as i8);
            } else {
                let d = rect_distance(x, y, n);
                let edge = r as f32 - 2.0 + 2.5 * noise(x as f32, y as f32, 0.15, 1, seed);
                if d > edge {
                    grid[xi][yi] = CellType::Sea;
                } else if d > edge - 1.3 {
                    grid[xi][yi] = CellType::Shore;
                } else {
                    let n2 = noise(x as f32, y as f32, 0.25, 2, seed);
                    let raw_h = (n2 * 3.0 - 0.8 + (1.0 - d / edge) * 0.8).round();
                    let mut h = (raw_h as i8).clamp(-1, 2);
                    if d < 3.0 {
                        h = h.min(0);
                    }
                    grid[xi][yi] = CellType::Grass(h);
                }
            }
        }
    }

    let cell_at = |gx: i32, gy: i32| -> CellType {
        if gx < min_coord || gx > max_coord || gy < min_coord || gy > max_coord {
            CellType::Sea
        } else {
            grid[(gx - min_coord) as usize][(gy - min_coord) as usize]
        }
    };

    let height_at = |cell: CellType| -> f32 {
        match cell {
            CellType::Board(h) | CellType::Grass(h) => top_y(h),
            CellType::Shore => top_y(-1),
            CellType::Sea => SEA_Y - 0.4,
        }
    };

    let mut solid = Quads::default();
    let mut decals = Quads::default();
    let mut sea_quads = [Quads::default(), Quads::default()];

    for x in min_coord..=max_coord {
        for y in min_coord..=max_coord {
            let cell = cell_at(x, y);
            let top_h = match cell {
                CellType::Grass(h) => top_y(h),
                CellType::Shore => top_y(-1),
                CellType::Board(_) | CellType::Sea => continue,
            };

            let cx = x as f32;
            let cz = -(y as f32);
            let top = Vec3::new(cx, top_h, cz);
            let corners = flat(top, Vec2::ONE);
            let n_shade = noise(cx, y as f32, 0.25, 2, seed);
            let shade = 0.68 * (0.92 + 0.08 * n_shade);

            match cell {
                CellType::Grass(_) => {
                    let tile = ["grass_0", "grass_1", "grass_dark", "moss"][hash(x, y, 1) as usize % 4];
                    solid.add(corners, full_uv(atlas.uv(tile)), shade);

                    if (hash(x, y, 200) % 100) < 30 {
                        let decal_idx = (hash(x, y, 201) % 8) as usize;
                        let decal_name = [
                            "decal_0", "decal_1", "decal_2", "decal_3", "decal_4", "decal_5", "decal_6",
                            "decal_7",
                        ][decal_idx];
                        let decal_rot = hash(x, y, 202) % 4;
                        let uv = rotate_uv(full_uv(atlas.uv(decal_name)), decal_rot);
                        let decal_corners = flat(top + Vec3::Y * 0.003, Vec2::splat(0.7));
                        decals.add(decal_corners, uv, 0.75);
                    }
                }
                CellType::Shore => {
                    let sea_dir = if cell_at(x, y + 1) == CellType::Sea {
                        Some(0)
                    } else if cell_at(x + 1, y) == CellType::Sea {
                        Some(1)
                    } else if cell_at(x, y - 1) == CellType::Sea {
                        Some(2)
                    } else if cell_at(x - 1, y) == CellType::Sea {
                        Some(3)
                    } else {
                        None
                    };

                    if let Some(dir) = sea_dir {
                        let uv_0 = rotate_uv(full_uv(atlas.uv("shore_0")), dir);
                        let uv_1 = rotate_uv(full_uv(atlas.uv("shore_1")), dir);
                        sea_quads[0].add(corners, uv_0, shade);
                        sea_quads[1].add(corners, uv_1, shade);
                    } else {
                        let name = if hash(x, y, 3).is_multiple_of(4) { "sand_shells" } else { "sand_dry" };
                        solid.add(corners, full_uv(atlas.uv(name)), shade);
                    }
                }
                CellType::Board(_) | CellType::Sea => unreachable!(),
            }

            for ((dx, dy), a, b, side_shade) in [
                ((0i8, -1i8), Vec3::new(cx - 0.5, 0.0, cz + 0.5), Vec3::new(cx + 0.5, 0.0, cz + 0.5), 0.84),
                ((1, 0), Vec3::new(cx + 0.5, 0.0, cz + 0.5), Vec3::new(cx + 0.5, 0.0, cz - 0.5), 0.72),
                ((0, 1), Vec3::new(cx + 0.5, 0.0, cz - 0.5), Vec3::new(cx - 0.5, 0.0, cz - 0.5), 0.56),
                ((-1, 0), Vec3::new(cx - 0.5, 0.0, cz - 0.5), Vec3::new(cx - 0.5, 0.0, cz + 0.5), 0.62),
            ] {
                let n_cell = cell_at(x + dx as i32, y + dy as i32);
                let n_top = height_at(n_cell);
                if n_top < top_h {
                    wall(&mut solid, &atlas, a, b, n_top, top_h, side_shade);
                }
            }

            if matches!(cell, CellType::Grass(_)) {
                let d = rect_distance(x, y, n);
                if d >= 1.5 {
                    let p = hash(x, y, 100) % 100;
                    let prop_name = if p < 10 {
                        (d >= 3.0).then_some("pine")
                    } else if p < 13 {
                        (d >= 3.0).then_some("dead_tree")
                    } else if p < 19 {
                        if hash(x, y, 101).is_multiple_of(2) { Some("rock") } else { Some("rock_mossy") }
                    } else if p < 25 {
                        Some("bush")
                    } else {
                        None
                    };

                    if let Some(prop) = prop_name {
                        let jx = ((hash(x, y, 102) as f32 / u32::MAX as f32) * 2.0 - 1.0) * 0.2;
                        let jz = ((hash(x, y, 103) as f32 / u32::MAX as f32) * 2.0 - 1.0) * 0.2;
                        let pos = top + Vec3::new(jx, 0.0, jz);
                        let mesh = card_mesh(&mut look, &mut meshes, &atlas, prop, false);
                        commands.spawn((
                            SceneryPart,
                            Billboard,
                            Mesh3d(mesh),
                            MeshMaterial3d(look.cards.clone()),
                            Transform::from_translation(pos),
                        ));
                    }
                }
            }
        }
    }

    commands.spawn((SceneryPart, Mesh3d(meshes.add(solid.mesh())), MeshMaterial3d(look.terrain.clone())));

    if !decals.pos.is_empty() {
        commands.spawn((SceneryPart, Mesh3d(meshes.add(decals.mesh())), MeshMaterial3d(look.cards.clone())));
    }

    let sea_min = -r - 40;
    let sea_max = n - 1 + r + 40;

    for x in sea_min..=sea_max {
        for y in sea_min..=sea_max {
            let cell = cell_at(x, y);
            if matches!(cell, CellType::Board(_) | CellType::Grass(_) | CellType::Shore) {
                continue;
            }

            let is_shallow =
                if x < min_coord - 2 || x > max_coord + 2 || y < min_coord - 2 || y > max_coord + 2 {
                    false
                } else {
                    let mut shallow = false;
                    'check: for dx in -2..=2 {
                        for dy in -2..=2 {
                            if matches!(
                                cell_at(x + dx, y + dy),
                                CellType::Board(_) | CellType::Grass(_) | CellType::Shore
                            ) {
                                shallow = true;
                                break 'check;
                            }
                        }
                    }
                    shallow
                };

            let corners = flat(Vec3::new(x as f32, SEA_Y, -(y as f32)), Vec2::ONE);
            let (frame_0, frame_1) =
                if is_shallow { ("sea_shallow_0", "sea_shallow_1") } else { ("sea_deep_0", "sea_deep_1") };
            sea_quads[0].add(corners, full_uv(atlas.uv(frame_0)), 1.0);
            sea_quads[1].add(corners, full_uv(atlas.uv(frame_1)), 1.0);
        }
    }

    let fine_min_x = sea_min as f32 - 0.5;
    let fine_max_x = sea_max as f32 + 0.5;
    let fine_min_z = -(sea_max as f32) - 0.5;
    let fine_max_z = -(sea_min as f32) + 0.5;
    let center = board_center(state.size);
    let (cx, cz) = (center.x, center.z);
    let k_fine = (sea_max - sea_min + 1) / 4;

    for k in -35..=(35 + k_fine) {
        let qx = fine_min_x - 2.0 + 4.0 * k as f32;
        if (qx - cx).abs() > 110.001 {
            continue;
        }
        for m in -35..=(35 + k_fine) {
            let qz = fine_min_z - 2.0 + 4.0 * m as f32;
            if (qz - cz).abs() > 110.001 {
                continue;
            }
            let overlaps = qx + 2.0 > fine_min_x + 1e-3
                && qx - 2.0 < fine_max_x - 1e-3
                && qz + 2.0 > fine_min_z + 1e-3
                && qz - 2.0 < fine_max_z - 1e-3;
            if overlaps {
                continue;
            }
            let corners = flat(Vec3::new(qx, SEA_Y, qz), Vec2::splat(4.0));
            sea_quads[0].add(corners, full_uv(atlas.uv("sea_deep_0")), 1.0);
            sea_quads[1].add(corners, full_uv(atlas.uv("sea_deep_1")), 1.0);
        }
    }

    for k in -35..=(35 + k_fine) {
        let qx = fine_min_x + 8.0 * k as f32;
        if (qx - cx).abs() > 215.001 {
            continue;
        }
        for m in -35..=(35 + k_fine) {
            let qz = fine_min_z + 8.0 * m as f32;
            if (qz - cz).abs() > 215.001 {
                continue;
            }
            if (qx - cx).abs() <= 110.001 && (qz - cz).abs() <= 110.001 {
                continue;
            }
            let corners = flat(Vec3::new(qx, SEA_Y, qz), Vec2::splat(8.0));
            sea_quads[0].add(corners, full_uv(atlas.uv("sea_deep_0")), 1.0);
            sea_quads[1].add(corners, full_uv(atlas.uv("sea_deep_1")), 1.0);
        }
    }

    for (i, q) in sea_quads.into_iter().enumerate() {
        let vis = if i == 0 { Visibility::Inherited } else { Visibility::Hidden };
        commands.spawn((
            SceneryPart,
            SeaFrame(i),
            vis,
            Mesh3d(meshes.add(q.mesh())),
            MeshMaterial3d(look.terrain.clone()),
        ));
    }
}

pub struct SceneryPlugin;

impl Plugin for SceneryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_sky).add_systems(
            Update,
            (
                rebuild_scenery,
                animate_sea,
                setup_birds,
                update_birds,
                update_clouds,
                update_sky_dome,
                update_camera_fog,
            ),
        );
    }
}
