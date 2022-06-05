use bevy::prelude::*;

use crate::spatial::Spatial;

pub struct BoundaryPlugin;

impl Plugin for BoundaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(boundary_remove_system)
                .with_system(boundary_wrap_system),
        );
    }
}

#[derive(Debug, Component, Default)]
pub struct BoundaryWrap;

#[derive(Debug, Component, Default)]
pub struct BoundaryRemoval;

fn boundary_wrap_system(
    window: Res<WindowDescriptor>,
    mut query: Query<(&mut Transform, &Spatial), With<BoundaryWrap>>,
) {
    for (mut transform, spatial) in query.iter_mut() {
        let x = transform.translation.x;
        let y = transform.translation.y;

        let half_width = window.width / 2.0;
        if x + spatial.radius * 2.0 < -half_width {
            transform.translation.x = half_width + spatial.radius * 2.0;
        } else if x - spatial.radius * 2.0 > half_width {
            transform.translation.x = -half_width - spatial.radius * 2.0;
        }

        let half_height = window.height / 2.0;
        if y + spatial.radius * 2.0 < -half_height {
            transform.translation.y = half_height + spatial.radius * 2.0;
        } else if y - spatial.radius * 2.0 > half_height {
            transform.translation.y = -half_height - spatial.radius * 2.0;
        }
    }
}

fn boundary_remove_system(
    window: Res<WindowDescriptor>,
    mut commands: Commands,
    query: Query<(Entity, &Transform, &Spatial), With<BoundaryRemoval>>,
) {
    for (entity, transform, spatial) in query.iter() {
        let half_width = window.width / 2.0;
        let half_height = window.height / 2.0;
        let x = transform.translation.x;
        let y = transform.translation.y;
        if x + spatial.radius * 2.0 < -half_width
            || x - spatial.radius * 2.0 > half_width
            || y + spatial.radius * 2.0 < -half_height
            || y - spatial.radius * 2.0 > half_height
        {
            commands.entity(entity).despawn();
        }
    }
}
