use std::time::Duration;

use bevy::prelude::*;

pub struct FlickPlugin;

impl Plugin for FlickPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (flick_removed_system, flick_system));
    }
}

#[derive(Debug, Component)]
pub struct Flick(Timer);

impl Flick {
    pub fn new(frequency: Duration) -> Self {
        Self(Timer::new(frequency, TimerMode::Repeating))
    }
}

fn flick_system(time: Res<Time>, mut query: Query<(&mut Flick, &mut Visibility)>) {
    for (mut flick, mut visibility) in query.iter_mut() {
        flick.0.tick(time.delta());

        if flick.0.finished() {
            *visibility = if *visibility == Visibility::Hidden {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        }
    }
}

fn flick_removed_system(mut removed: RemovedComponents<Flick>, mut query: Query<&mut Visibility>) {
    for entity in removed.read() {
        if let Ok(mut visibility) = query.get_mut(entity) {
            *visibility = Visibility::Inherited
        }
    }
}
