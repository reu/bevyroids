use std::marker::PhantomData;

use bevy::prelude::*;

use crate::spatial::Spatial;

pub struct CollisionPlugin<Hittable, Hurtable> {
    _phantom: PhantomData<(Hittable, Hurtable)>,
}

impl<Hittable: Component, Hurtable: Component> CollisionPlugin<Hittable, Hurtable> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<Hittable: Component, Hurtable: Component> Plugin for CollisionPlugin<Hittable, Hurtable> {
    fn build(&self, app: &mut App) {
        app.add_event::<HitEvent<Hittable, Hurtable>>()
            .add_system(collision_system::<Hittable, Hurtable>.label(CollisionSystemLabel));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(SystemLabel)]
pub struct CollisionSystemLabel;

#[derive(Debug)]
pub struct HitEvent<A, B> {
    entities: (Entity, Entity),
    _phantom: PhantomData<(A, B)>,
}

impl<A, B> HitEvent<A, B> {
    pub fn hittable(&self) -> Entity {
        self.entities.0
    }

    pub fn hurtable(&self) -> Entity {
        self.entities.1
    }
}

#[derive(Debug, Component)]
pub struct Collidable;

fn collision_system<A: Component, B: Component>(
    mut hits: EventWriter<HitEvent<A, B>>,
    hittables: Query<(Entity, &Spatial), (With<Collidable>, With<A>)>,
    hurtables: Query<(Entity, &Spatial), (With<Collidable>, With<B>)>,
) {
    for (hittable_entity, hittable) in hittables.iter() {
        for (hurtable_entity, hurtable) in hurtables.iter() {
            if hittable.intersects(hurtable) {
                hits.send(HitEvent {
                    entities: (hittable_entity, hurtable_entity),
                    _phantom: PhantomData,
                });
            }
        }
    }
}
