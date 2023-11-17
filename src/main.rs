use std::f32::consts::PI;

use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};
use noise::{NoiseFn, Perlin};

fn main() {
    App::new()
        .init_resource::<Noise>()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (ai_tank_update, camera_update, cannonball_update))
        .run();
}

#[derive(Component)]
pub struct AiTank {
    /// This id seeds the noise function used for movement
    id: u32,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct PlayerTank;

#[derive(Component)]
pub struct Velocity {
    val: Vec3,
}

#[derive(Resource, Default)]
pub struct Noise {
    generator: Perlin,
}

fn tank_color(tank_id: u32) -> Color {
    let hue = (tank_id % 20) as f32 * 18.0;
    let x = 1.0 - ((hue / 60.0) % 2.0 - 1.0).abs();

    if hue < 60.0 {
        Color::rgb(1.0, x, 0.0)
    } else if hue < 120.0 {
        Color::rgb(x, 1.0, 0.0)
    } else if hue < 180.0 {
        Color::rgb(0.0, 1.0, x)
    } else if hue < 240.0 {
        Color::rgb(0.0, x, 1.0)
    } else if hue < 300.0 {
        Color::rgb(x, 0.0, 1.0)
    } else {
        Color::rgb(1.0, 0.0, x)
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // sun

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::default().looking_at(Vec3::new(0.717, -0.717, 0.0), Vec3::Y),
        ..default()
    });

    // camera

    commands.spawn(Camera3dBundle::default());

    // floor

    commands.spawn((PbrBundle {
        mesh: asset_server.load("cube.glb#Mesh0/Primitive0"),
        material: materials.add(Color::rgb(0.8, 0.8, 0.8).into()),
        transform: Transform {
            translation: Vec3::new(0.0, -0.5, 0.0),
            scale: Vec3::new(200.0, 1.0, 200.0),
            ..Default::default()
        },
        ..default()
    },));

    // spawn player tank

    commands.spawn((
        PbrBundle {
            mesh: asset_server.load("tank.glb#Mesh0/Primitive0"),
            material: materials.add(tank_color(0).into()),
            ..default()
        },
        PlayerTank,
    ));

    // spawn AI tanks

    for id in 1..20 {
        let material = materials.add(tank_color(id).into());
        commands.spawn((
            PbrBundle {
                mesh: asset_server.load("tank.glb#Mesh0/Primitive0"),
                material: material.clone(),
                ..default()
            },
            AiTank { id, material },
        ));
    }
}

fn ai_tank_update(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    noise: Res<Noise>,
    mut query: Query<(&AiTank, &mut Transform)>,
) {
    for (tank, mut transform) in &mut query {
        // Update the tank transform based on a perlin noise function.

        let seed = transform.translation / 10.0;
        let noise = noise
            .generator
            .get([seed.x as f64, tank.id as f64, seed.z as f64]) as f32;
        let angle = (0.5 + noise) * 4.0 * PI;

        let tank_direction = Vec3::new(angle.sin(), 0.0, angle.cos());

        transform.translation += tank_direction * time.delta_seconds() * 5.0;
        transform.rotation = Quat::from_axis_angle(Vec3::Y, angle);

        // Shoot one cannonball per frame.

        spawn_cannonball(
            &mut commands,
            &asset_server,
            &transform,
            tank.material.clone(),
        );
    }
}

fn spawn_cannonball(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    tank_transform: &Transform,
    material: Handle<StandardMaterial>,
) {
    // Shoot from the tip of the cannon, which is (0.0, 1.235, 0.324) in local coordinates
    let offset = tank_transform
        .rotation
        .mul_vec3(Vec3::new(0.0, 1.235, 0.324));

    let transform = Transform {
        translation: tank_transform.translation + offset,
        rotation: tank_transform.rotation,
        scale: Vec3::new(0.2, 0.2, 0.2),
    };

    let velocity = Velocity {
        val: tank_transform
            .rotation
            .mul_vec3(Vec3::new(0.0, 0.717, 0.8) * 20.0),
    };

    commands.spawn((
        PbrBundle {
            mesh: asset_server.load("sphere.glb#Mesh0/Primitive0"),
            material,
            transform,
            ..default()
        },
        velocity,
    ));
}

fn cannonball_update(
    par_commands: ParallelCommands,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Velocity, Entity)>,
) {
    query
        .par_iter_mut()
        .for_each(|(mut transform, mut velocity, entity)| {
            // Move cannonball by the current velocity.

            transform.translation += velocity.val * time.delta_seconds();

            // Bounce if position drops below floor.

            if transform.translation.y < 0.1 {
                transform.translation.y += 0.1 - transform.translation.y;

                let damping = Vec3::new(0.8, -0.8, 0.8);
                velocity.val *= damping;
            }

            // Acceleration due to gravity.

            velocity.val.y -= 9.82 * time.delta_seconds();

            // Despawn if velocity drops low enough.

            if velocity.val.length_squared() < 0.1 {
                par_commands.command_scope(|mut commands| {
                    commands.entity(entity).despawn();
                });
            }
        });
}

fn camera_update(
    mut query_camera: Query<&mut Transform, With<Camera>>,
    query_player_tank: Query<&Transform, (With<PlayerTank>, Without<Camera>)>,
) {
    let tank_transform = query_player_tank.get_single().unwrap();
    *query_camera.single_mut() = camera_transform(tank_transform);
}

fn camera_transform(tank_transform: &Transform) -> Transform {
    // Position the camera above and behind the player tank.

    let camera_local_translation = tank_transform.rotation.mul_vec3(Vec3::new(0.0, 5.0, -10.0));

    let translation = tank_transform.translation + camera_local_translation;
    let target = tank_transform.translation + Vec3::Y;

    Transform {
        translation,
        ..Default::default()
    }
    .looking_at(target, Vec3::Y)
}
