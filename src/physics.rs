use std::time::Duration;

use bevy::{ecs::schedule::ScheduleLabel, prelude::*, time::common_conditions::on_timer};
use derive_more::From;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet, ScheduleLabel)]
pub struct PhysicsSystemLabel;

#[derive(Resource)]
pub struct TimeStep(pub f32);

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeStep(self.time_step)).add_systems(
            FixedUpdate,
            (
                damping_system.before(movement_system),
                speed_limit_system.before(movement_system),
                movement_system,
            )
                .distributive_run_if(on_timer(Duration::from_secs_f32(self.time_step)))
                .in_set(PhysicsSystemLabel),
        );
    }
}

#[derive(Debug, Component, Default, Deref, DerefMut, From, Resource)]
pub struct Velocity(Vec2);

#[derive(Debug, Component, Default, Deref, DerefMut, From, Resource)]
pub struct AngularVelocity(f32);

#[derive(Debug, Component, Default, Deref, DerefMut, From)]
pub struct SpeedLimit(f32);

#[derive(Debug, Component, Default, Deref, DerefMut, From)]
pub struct Damping(f32);

fn movement_system(
    time_step: Res<TimeStep>,
    mut query: Query<(&mut Transform, Option<&Velocity>, Option<&AngularVelocity>)>,
) {
    for (mut transform, velocity, angular_velocity) in query.iter_mut() {
        if let Some(Velocity(vel)) = velocity {
            transform.translation.x += vel.x * time_step.0;
            transform.translation.y += vel.y * time_step.0;
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
