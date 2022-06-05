use bevy::prelude::*;

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Component, Default, Clone)]
pub struct Spatial {
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
pub struct SpatialSystemLabel;
