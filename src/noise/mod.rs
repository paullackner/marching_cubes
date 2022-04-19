use bevy::{prelude::*, render::renderer::{RenderDevice, RenderQueue}};

use self::opensimplex::*;

pub mod opensimplex;

pub struct NoisePlugin;

impl Plugin for NoisePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<OpenSimplex>()
        ;
    }
}