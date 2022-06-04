use std::time::Duration;

use bevy::prelude::*;

pub struct FlickPlugin;

impl Plugin for FlickPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PostUpdate, flick_removed_system)
            .add_system(flick_system);
    }
}

#[derive(Debug, Component)]
pub struct Flick(Timer);

impl Flick {
    pub fn new(frequency: Duration) -> Self {
        Self(Timer::new(frequency, true))
    }
}

fn flick_system(time: Res<Time>, mut query: Query<(&mut Flick, &mut Visibility)>) {
    for (mut flick, mut visibility) in query.iter_mut() {
        flick.0.tick(time.delta());

        if flick.0.finished() {
            visibility.is_visible = !visibility.is_visible;
        }
    }
}

fn flick_removed_system(removed: RemovedComponents<Flick>, mut query: Query<&mut Visibility>) {
    for entity in removed.iter() {
        if let Ok(mut visibility) = query.get_mut(entity) {
            visibility.is_visible = true;
        }
    }
}
