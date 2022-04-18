use bevy::{prelude::*, render::renderer::{RenderDevice, RenderQueue}};

use self::opensimplex::*;

pub mod opensimplex;

pub struct NoisePlugin;

impl Plugin for NoisePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<OpenSimplex>()
            // .add_startup_system(test)
        ;
    }
}

fn test(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    simplex: Res<OpenSimplex>,
) {
    let points = simplex.compute_chunk(Vec3::new(0.0, 0.0, 0.0), &render_device, &render_queue);

    println!("{:?}", points);
}