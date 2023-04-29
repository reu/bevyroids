use std::time::Duration;

use bevy::prelude::*;

pub struct ExpirationPlugin;

impl Plugin for ExpirationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(expiration_system.in_base_set(CoreSet::PostUpdate));
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct Expiration(Timer);

impl Expiration {
    pub fn new(duration: Duration) -> Self {
        Self(Timer::new(duration, TimerMode::Once))
    }
}

fn expiration_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Expiration)>,
) {
    for (entity, mut expiration) in query.iter_mut() {
        expiration.0.tick(time.delta());

        if expiration.0.finished() {
            commands.entity(entity).despawn();
        }
    }
}
