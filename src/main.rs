mod world;

use bevy::{prelude::*, render::{render_resource::{PrimitiveTopology, WgpuFeatures}, renderer::RenderDevice, options::WgpuOptions}, pbr::wireframe::{WireframePlugin, WireframeConfig, Wireframe}};
use bevy_fly_camera::{self, FlyCamera, FlyCameraPlugin};
use world::chunk::ChunkPlugin;

use crate::world::chunk::{ChunkBundle, Chunk, AXIS_SIZE, DirtyChunk};

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WgpuOptions {
            features: WgpuFeatures::POLYGON_MODE_LINE,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(WireframePlugin)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(ChunkPlugin)
        .add_startup_system(setup)
        .add_system(cursor_grab_system)
        // .add_system(terrain_edit)
        .run();
}


fn setup(
    mut commands: Commands,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    wireframe_config.global = false;
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

    for x in 0..=1 {
        for y in 0..=0 {
            for z in 0..=1 {
                commands.spawn_bundle(ChunkBundle {
                    chunk: Chunk::new_empty(),
                    pbr: PbrBundle {
                        mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
                        transform: Transform::from_xyz(((AXIS_SIZE-1) * x) as f32, ((AXIS_SIZE-1) * y) as f32, ((AXIS_SIZE-1) * z) as f32),
                        material: materials.add(Color::DARK_GREEN.into()),
                        ..Default::default()
                    }
                })
                .insert(Wireframe)
                .insert(DirtyChunk);
                
            }
        }
    }
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