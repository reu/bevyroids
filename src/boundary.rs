use bevy::prelude::*;

use crate::spatial::Spatial;

pub struct BoundaryPlugin;

impl Plugin for BoundaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
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
    mut query: Query<&mut Spatial, With<BoundaryWrap>>,
) {
    for mut spatial in query.iter_mut() {
        let half_width = window.width / 2.0;
        if spatial.position.x + spatial.radius * 2.0 < -half_width {
            spatial.position.x = half_width + spatial.radius * 2.0;
        } else if spatial.position.x - spatial.radius * 2.0 > half_width {
            spatial.position.x = -half_width - spatial.radius * 2.0;
        }

        let half_height = window.height / 2.0;
        if spatial.position.y + spatial.radius * 2.0 < -half_height {
            spatial.position.y = half_height + spatial.radius * 2.0;
        } else if spatial.position.y - spatial.radius * 2.0 > half_height {
            spatial.position.y = -half_height - spatial.radius * 2.0;
        }
    }
}

fn boundary_remove_system(
    window: Res<WindowDescriptor>,
    mut commands: Commands,
    query: Query<(Entity, &Spatial), With<BoundaryRemoval>>,
) {
    for (entity, spatial) in query.iter() {
        let half_width = window.width / 2.0;
        let half_height = window.height / 2.0;
        if spatial.position.x + spatial.radius * 2.0 < -half_width
            || spatial.position.x - spatial.radius * 2.0 > half_width
            || spatial.position.y + spatial.radius * 2.0 < -half_height
            || spatial.position.y - spatial.radius * 2.0 > half_height
        {
            commands.entity(entity).despawn();
        }
    }
}
