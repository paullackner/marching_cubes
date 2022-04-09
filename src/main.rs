mod world;

use bevy::{prelude::*, render::render_resource::PrimitiveTopology};
use bevy_fly_camera::{self, FlyCamera, FlyCameraPlugin};
use world::chunk::{ChunkPlugin, ChunkBundle, Chunk, AXIS_SIZE};
use bevy_mod_raycast::{DefaultPluginState, DefaultRaycastingPlugin, RayCastMesh, RayCastSource};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(DefaultRaycastingPlugin::<ChunkRayCastSet>::default())
        .add_plugin(ChunkPlugin)
        .add_startup_system(setup)
        .add_system(cursor_grab_system)
        // .add_system(terrain_edit)
        .run();
}

struct ChunkRayCastSet;

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
    .insert(RayCastSource::<ChunkRayCastSet>::new_transform_empty())
    .insert(FlyCamera::default());


    for x in 0..=20 {
        for y in 0..=3 {
            for z in 0..=20 {
                commands.spawn_bundle(ChunkBundle {
                    chunk: Chunk::new_empty(),
                    pbr: PbrBundle {
                        mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
                        transform: Transform::from_xyz(((AXIS_SIZE-1) * x) as f32, ((AXIS_SIZE-1) * y) as f32, ((AXIS_SIZE-1) * z) as f32),
                        material: materials.add(Color::DARK_GREEN.into()),
                        ..Default::default()
                    }
                });
                
            }
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

fn terrain_edit(
    time: Res<Time>, 
    mut query: Query<&mut Transform, With<RayCastSource<ChunkRayCastSet>>>,
    // btn: Res<Input<MouseButton>>,
) {
    for mut transform in query.iter_mut() {
        *transform = Transform::from_rotation(
            Quat::from_rotation_x(time.seconds_since_startup().sin() as f32 * 0.15)
                * Quat::from_rotation_y((time.seconds_since_startup() * 1.5).sin() as f32 * 0.1),
        );
    }
}