use bevy::{core::FixedTimestep, prelude::*};
use derive_more::From;

use crate::spatial::Spatial;

pub struct PhysicsPlugin {
    time_step: f32,
}

impl PhysicsPlugin {
    pub fn with_fixed_time_step(time_step: f32) -> Self {
        Self { time_step }
    }
}

impl Default for PhysicsPlugin {
    fn default() -> Self {
        Self::with_fixed_time_step(1.0 / 60.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
pub struct PhysicsSystemLabel;

pub struct TimeStep(pub f32);

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeStep(self.time_step))
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(self.time_step.into()))
                    .label(PhysicsSystemLabel)
                    .with_system(damping_system.before(movement_system))
                    .with_system(speed_limit_system.before(movement_system))
                    .with_system(movement_system),
            );
    }
}

#[derive(Debug, Component, Default, Deref, DerefMut, From)]
pub struct Velocity(Vec2);

#[derive(Debug, Component, Default, Deref, DerefMut, From)]
pub struct AngularVelocity(f32);

#[derive(Debug, Component, Default, Deref, DerefMut, From)]
pub struct SpeedLimit(f32);

#[derive(Debug, Component, Default, Deref, DerefMut, From)]
pub struct Damping(f32);

fn movement_system(
    time_step: Res<TimeStep>,
    mut query: Query<(
        &mut Spatial,
        &mut Transform,
        Option<&Velocity>,
        Option<&AngularVelocity>,
    )>,
) {
    for (mut spatial, mut transform, velocity, angular_velocity) in query.iter_mut() {
        if let Some(velocity) = velocity {
            spatial.position += velocity.0 * time_step.0;
        }
        if let Some(AngularVelocity(vel)) = angular_velocity {
            transform.rotate(Quat::from_rotation_z(vel * time_step.0));
        }
    }
}

fn speed_limit_system(mut query: Query<(&mut Velocity, &SpeedLimit)>) {
    for (mut velocity, speed_limit) in query.iter_mut() {
        velocity.0 = velocity.0.clamp_length_max(speed_limit.0);
    }
}

fn damping_system(mut query: Query<(&mut Velocity, &Damping)>) {
    for (mut velocity, damping) in query.iter_mut() {
        velocity.0 *= damping.0;
    }
}
