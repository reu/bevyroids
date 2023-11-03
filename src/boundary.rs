use bevy::{prelude::*, window::PrimaryWindow};

pub struct BoundaryPlugin;

impl Plugin for BoundaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (boundary_remove_system, boundary_wrap_system));
    }
}

#[derive(Debug, Component, Default, Clone, Copy, Deref, DerefMut)]
pub struct Bounding(f32);

impl Bounding {
    pub fn from_radius(radius: f32) -> Self {
        Self(radius)
    }
}

#[derive(Debug, Component, Default)]
pub struct BoundaryWrap;

#[derive(Debug, Component, Default)]
pub struct BoundaryRemoval;

fn boundary_wrap_system(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut Transform, &Bounding), With<BoundaryWrap>>,
) {
    if let Ok(window) = primary_window.get_single() {
        for (mut transform, radius) in query.iter_mut() {
            let x = transform.translation.x;
            let y = transform.translation.y;

            let half_width = window.width() / 2.0;
            if x + radius.0 * 2.0 < -half_width {
                transform.translation.x = half_width + radius.0 * 2.0;
            } else if x - radius.0 * 2.0 > half_width {
                transform.translation.x = -half_width - radius.0 * 2.0;
            }

            let half_height = window.height() / 2.0;
            if y + radius.0 * 2.0 < -half_height {
                transform.translation.y = half_height + radius.0 * 2.0;
            } else if y - radius.0 * 2.0 > half_height {
                transform.translation.y = -half_height - radius.0 * 2.0;
            }
        }
    }
}

fn boundary_remove_system(
    mut commands: Commands,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    query: Query<(Entity, &Transform, &Bounding), With<BoundaryRemoval>>,
) {
    if let Ok(window) = primary_window.get_single() {
        for (entity, transform, radius) in query.iter() {
            let half_width = window.width() / 2.0;
            let half_height = window.height() / 2.0;
            let x = transform.translation.x;
            let y = transform.translation.y;
            if x + radius.0 * 2.0 < -half_width
                || x - radius.0 * 2.0 > half_width
                || y + radius.0 * 2.0 < -half_height
                || y - radius.0 * 2.0 > half_height
            {
                commands.entity(entity).despawn();
            }
        }
    }
}
