mod world;

use bevy::{prelude::*, render::render_resource::PrimitiveTopology};
use bevy_fly_camera::{self, FlyCamera, FlyCameraPlugin};
use world::chunk::{ChunkPlugin, ChunkBundle, Chunk, AXIS_SIZE};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(ChunkPlugin)
        .add_startup_system(setup)
        .add_system(cursor_grab_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0})),
        transform: Transform::from_xyz(0.0, 0.0, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
        material: materials.add(Color::RED.into()),
        ..Default::default()
    });

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    })
    .insert(FlyCamera::default());


    for i in 0..=10 {
        for j in 0..=10 {
            commands.spawn_bundle(ChunkBundle {
                chunk: Chunk::new_empty(),
                pbr: PbrBundle {
                    mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
                    transform: Transform::from_xyz(((AXIS_SIZE-1) * i) as f32, 0.0, ((AXIS_SIZE-1) * j) as f32),
                    material: materials.add(Color::DARK_GREEN.into()),
                    ..Default::default()
                }
            });
        }
    }

    const HALF_SIZE: f32 = 10.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // Configure the projection to better fit the scene
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..Default::default()
            },
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..Default::default()
        },
        ..Default::default()
    });
}

fn cursor_grab_system(
    mut windows: ResMut<Windows>,
    btn: Res<Input<MouseButton>>,
    key: Res<Input<KeyCode>>,
) {
    let window = windows.get_primary_mut().unwrap();

    if btn.just_pressed(MouseButton::Left) {
        window.set_cursor_lock_mode(true);
        window.set_cursor_visibility(false);
    }

    if key.just_pressed(KeyCode::Escape) {
        window.set_cursor_lock_mode(false);
        window.set_cursor_visibility(true);
    }
}
