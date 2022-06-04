use bevy::prelude::*;

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spatial_system.label(SpatialSystemLabel));
    }
}

#[derive(Debug, Component, Default, Clone)]
pub struct Spatial {
    pub position: Vec2,
    pub rotation: f32,
    pub radius: f32,
}

impl Spatial {
    pub fn intersects(&self, other: &Spatial) -> bool {
        let distance = (self.position - other.position).length();
        distance < self.radius + other.radius
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
pub struct SpatialSystemLabel;

fn spatial_system(mut query: Query<(&mut Transform, &Spatial)>) {
    for (mut transform, spatial) in query.iter_mut() {
        transform.translation.x = spatial.position.x;
        transform.translation.y = spatial.position.y;
        transform.rotation = Quat::from_rotation_z(spatial.rotation);
    }
}
