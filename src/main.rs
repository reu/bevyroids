#![allow(clippy::type_complexity)]

use std::{f32::consts::PI, ops::Range, time::Duration};

use bevy::{core::FixedTimestep, prelude::*, window::PresentMode};
use bevy_prototype_lyon::{
    entity::ShapeBundle,
    prelude::{
        tess::{geom::Rotation, math::Angle},
        *,
    },
    shapes::Polygon,
};
use collision::{Collidable, CollisionPlugin, HitEvent, CollisionSystemLabel};
use rand::{prelude::SmallRng, Rng, SeedableRng};
use spatial::{Spatial, SpatialPlugin};

mod collision;
mod spatial;

const TIME_STEP: f32 = 1.0 / 120.0;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Bevyroids".to_string(),
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .insert_resource(Random(SmallRng::from_entropy()))
        .insert_resource(AsteroidSizes {
            big: 50.0..60.0,
            medium: 30.0..40.0,
            small: 10.0..20.0,
        })
        .add_event::<AsteroidSpawnEvent>()
        .add_plugin(SpatialPlugin)
        .add_plugin(CollisionPlugin::<Bullet, Asteroid>::new())
        .add_plugin(CollisionPlugin::<Asteroid, Ship>::new())
        .add_startup_system(setup_system)
        .add_system_set(
            SystemSet::new()
                .label("input")
                .with_system(steering_control_system)
                .with_system(thrust_control_system)
                .with_system(weapon_control_system),
        )
        .add_system(weapon_system.after("input").before("physics"))
        .add_system(thrust_system.after("input").before("physics"))
        .add_system(asteroid_spawn_system.with_run_criteria(FixedTimestep::step(0.5)))
        .add_system(asteroid_generation_system)
        .add_system(expiration_system)
        .add_system(explosion_system)
        .add_system(flick_system)
        .add_system_to_stage(CoreStage::PostUpdate, flick_removed_system)
        .add_system(ship_state_system)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP.into()))
                .label("physics")
                .after("input")
                .with_system(damping_system.before(movement_system))
                .with_system(speed_limit_system.before(movement_system))
                .with_system(movement_system),
        )
        .add_system_set(
            SystemSet::new()
                .label("wrap")
                .after("physics")
                .with_system(boundary_remove_system)
                .with_system(boundary_wrap_system),
        )
        .add_system(asteroid_hit_system.after(CollisionSystemLabel))
        .add_system(ship_hit_system.after(CollisionSystemLabel))
        .run();
}

#[derive(Debug, Deref, DerefMut)]
struct Random(SmallRng);

impl FromWorld for Random {
    fn from_world(world: &mut World) -> Self {
        let rng = world
            .get_resource_mut::<Random>()
            .expect("Random resource not found");
        Random(SmallRng::from_rng(rng.clone()).unwrap())
    }
}

#[derive(Debug, Clone)]
struct AsteroidSizes {
    big: Range<f32>,
    medium: Range<f32>,
    small: Range<f32>,
}

#[derive(Debug, Component, Default)]
struct Velocity(Vec2);

#[derive(Debug, Component, Default)]
struct AngularVelocity(f32);

#[derive(Debug, Component, Default)]
struct SpeedLimit(f32);

#[derive(Debug, Component, Default)]
struct Damping(f32);

#[derive(Debug, Component, Default)]
struct ThrustEngine {
    force: f32,
    on: bool,
}

impl ThrustEngine {
    pub fn new(force: f32) -> Self {
        Self {
            force,
            ..Default::default()
        }
    }
}

#[derive(Debug, Component, Default)]
struct SteeringControl(Angle);

#[derive(Debug, Component, Default)]
struct Weapon {
    cooldown: Timer,
    triggered: bool,
}

impl Weapon {
    pub fn new(rate_of_fire: Duration) -> Self {
        Self {
            cooldown: Timer::new(rate_of_fire, true),
            ..Default::default()
        }
    }
}

#[derive(Debug, Component)]
struct Expiration(Timer);

impl Expiration {
    pub fn new(duration: Duration) -> Self {
        Self(Timer::new(duration, false))
    }
}

#[derive(Debug, Component)]
struct Flick(Timer);

impl Flick {
    pub fn new(frequency: Duration) -> Self {
        Self(Timer::new(frequency, true))
    }
}

#[derive(Debug, Component, Default)]
struct BoundaryWrap;

#[derive(Debug, Component, Default)]
struct BoundaryRemoval;

#[derive(Debug, Component, Default)]
pub struct Ship {
    state: ShipState,
}

impl Ship {
    fn alive() -> Self {
        Ship {
            state: ShipState::Alive,
        }
    }

    fn dead(duration: Duration) -> Self {
        Ship {
            state: ShipState::Dead(Timer::new(duration, false)),
        }
    }

    fn spawn(duration: Duration) -> Self {
        Ship {
            state: ShipState::Spawning(Timer::new(duration, false)),
        }
    }
}

#[derive(Debug)]
enum ShipState {
    Alive,
    Dead(Timer),
    Spawning(Timer),
}

impl Default for ShipState {
    fn default() -> Self {
        ShipState::Alive
    }
}

#[derive(Debug, Component, Default)]
pub struct Bullet;

#[derive(Debug, Component, Default)]
struct Explosion;

#[derive(Debug, Component, Default)]
pub struct Asteroid;

#[derive(Debug, Deref)]
struct AsteroidSpawnEvent(Spatial);

#[derive(Bundle)]
struct ExplosionBundle {
    #[bundle]
    shape_bundle: ShapeBundle,
    explosion: Explosion,
    spatial: Spatial,
    velocity: Velocity,
    damping: Damping,
    expiration: Expiration,
}

impl Default for ExplosionBundle {
    fn default() -> Self {
        Self {
            shape_bundle: GeometryBuilder::build_as(
                &shapes::Circle {
                    radius: 1.0,
                    center: Vec2::ZERO,
                },
                DrawMode::Fill(FillMode::color(Color::BLACK)),
                Transform::default(),
            ),
            explosion: Explosion::default(),
            spatial: Spatial::default(),
            velocity: Velocity::default(),
            damping: Damping(0.97),
            expiration: Expiration::new(Duration::from_secs(1)),
        }
    }
}

fn setup_system(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn().insert(Ship::spawn(Duration::from_secs(0)));
}

fn movement_system(mut query: Query<(&mut Spatial, Option<&Velocity>, Option<&AngularVelocity>)>) {
    for (mut spatial, velocity, angular_velocity) in query.iter_mut() {
        if let Some(velocity) = velocity {
            spatial.position += velocity.0 * TIME_STEP;
        }
        if let Some(angular_velocity) = angular_velocity {
            spatial.rotation += angular_velocity.0 * TIME_STEP;
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

fn thrust_system(mut query: Query<(&mut Velocity, &ThrustEngine, &Spatial)>) {
    for (mut velocity, thrust, spatial) in query.iter_mut() {
        if thrust.on {
            velocity.0.x += spatial.rotation.cos() * thrust.force;
            velocity.0.y += spatial.rotation.sin() * thrust.force;
        }
    }
}

fn weapon_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(&Spatial, &mut Weapon)>,
) {
    for (spatial, mut weapon) in query.iter_mut() {
        weapon.cooldown.tick(time.delta());

        if weapon.cooldown.finished() && weapon.triggered {
            weapon.triggered = false;

            let bullet_dir = Vec2::new(spatial.rotation.cos(), spatial.rotation.sin());
            let bullet_vel = bullet_dir * 1000.0;
            let bullet_pos = spatial.position + (bullet_dir * spatial.radius);

            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shapes::Circle {
                        radius: 2.0,
                        center: Vec2::ZERO,
                    },
                    DrawMode::Fill(FillMode::color(Color::BLACK)),
                    Transform::default().with_translation(Vec3::new(
                        bullet_pos.x,
                        bullet_pos.y,
                        0.0,
                    )),
                ))
                .insert(Bullet)
                .insert(Collidable)
                .insert(Spatial {
                    position: bullet_pos,
                    rotation: 0.0,
                    radius: 2.0,
                })
                .insert(Velocity(bullet_vel))
                .insert(BoundaryRemoval);
        }
    }
}

fn ship_state_system(
    time: Res<Time>,
    mut commands: Commands,
    mut ships: Query<(Entity, &mut Ship)>,
) {
    for (entity, mut ship) in ships.iter_mut() {
        match ship.state {
            ShipState::Alive => {}

            ShipState::Dead(ref mut timer) => {
                if timer.elapsed().is_zero() {
                    commands
                        .entity(entity)
                        .remove_bundle::<ShapeBundle>()
                        .remove::<SteeringControl>()
                        .remove::<Weapon>()
                        .remove::<ThrustEngine>()
                        .remove::<Collidable>();
                }

                timer.tick(time.delta());

                if timer.finished() {
                    *ship = Ship::spawn(Duration::from_secs(2));
                }
            }

            ShipState::Spawning(ref mut timer) => {
                if timer.elapsed().is_zero() {
                    commands
                        .entity(entity)
                        .insert_bundle(GeometryBuilder::build_as(
                            &{
                                let mut path_builder = PathBuilder::new();
                                path_builder.move_to(Vec2::ZERO);
                                path_builder.line_to(Vec2::new(-8.0, -8.0));
                                path_builder.line_to(Vec2::new(0.0, 12.0));
                                path_builder.line_to(Vec2::new(8.0, -8.0));
                                path_builder.line_to(Vec2::ZERO);
                                let mut line = path_builder.build();
                                line.0 = line.0.transformed(&Rotation::new(Angle::degrees(-90.0)));
                                line
                            },
                            DrawMode::Stroke(StrokeMode::new(Color::BLACK, 1.0)),
                            Transform::default(),
                        ))
                        .insert(Spatial {
                            position: Vec2::ZERO,
                            rotation: 0.0,
                            radius: 12.0,
                        })
                        .insert(Velocity::default())
                        .insert(SpeedLimit(350.0))
                        .insert(Damping(0.998))
                        .insert(ThrustEngine::new(1.5))
                        .insert(AngularVelocity::default())
                        .insert(SteeringControl(Angle::degrees(180.0)))
                        .insert(BoundaryWrap)
                        .insert(Flick::new(Duration::from_millis(80)));
                }

                timer.tick(time.delta());

                if timer.finished() {
                    *ship = Ship::alive();

                    commands
                        .entity(entity)
                        .insert(Weapon::new(Duration::from_millis(100)))
                        .insert(Collidable)
                        .remove::<Flick>();
                }
            }
        }
    }
}

fn asteroid_spawn_system(
    window: Res<WindowDescriptor>,
    asteroid_sizes: Res<AsteroidSizes>,
    mut rng: Local<Random>,
    mut asteroids: EventWriter<AsteroidSpawnEvent>,
) {
    if rng.gen_bool(1.0 / 3.0) {
        let w = window.width / 2.0;
        let h = window.height / 2.0;

        let x = rng.gen_range(-w..w);
        let y = rng.gen_range(-h..h);
        let radius = match rng.gen_range(1..=3) {
            3 => rng.gen_range(asteroid_sizes.big.clone()),
            2 => rng.gen_range(asteroid_sizes.medium.clone()),
            _ => rng.gen_range(asteroid_sizes.small.clone()),
        };
        let c = radius * 2.0;

        let position = if rng.gen_bool(1.0 / 2.0) {
            Vec2::new(x, if y > 0.0 { h + c } else { -h - c })
        } else {
            Vec2::new(if x > 0.0 { w + c } else { -w - c }, y)
        };

        asteroids.send(AsteroidSpawnEvent(Spatial {
            position,
            radius,
            ..Default::default()
        }));
    }
}

fn asteroid_generation_system(
    window: Res<WindowDescriptor>,
    asteroid_sizes: Res<AsteroidSizes>,
    mut rng: Local<Random>,
    mut asteroids: EventReader<AsteroidSpawnEvent>,
    mut commands: Commands,
) {
    let w = window.width / 2.0;
    let h = window.height / 2.0;

    for asteroid in asteroids.iter() {
        let position = asteroid.position;

        let velocity = Vec2::new(rng.gen_range(-w..w), rng.gen_range(-h..h));
        let scale = if asteroid_sizes.big.contains(&asteroid.radius) {
            rng.gen_range(30.0..60.0)
        } else if asteroid_sizes.medium.contains(&asteroid.radius) {
            rng.gen_range(60.0..80.0)
        } else {
            rng.gen_range(80.0..100.0)
        };
        let velocity = (velocity - position).normalize_or_zero() * scale;

        let shape = {
            let sides = rng.gen_range(6..12);
            let mut points = Vec::with_capacity(sides);
            let n = sides as f32;
            let internal = (n - 2.0) * PI / n;
            let offset = -internal / 2.0;
            let step = 2.0 * PI / n;
            let r = asteroid.radius;
            for i in 0..sides {
                let cur_angle = (i as f32).mul_add(step, offset);
                let x = r * rng.gen_range(0.5..1.2) * cur_angle.cos();
                let y = r * rng.gen_range(0.5..1.2) * cur_angle.sin();
                points.push(Vec2::new(x, y));
            }
            Polygon {
                points,
                closed: true,
            }
        };

        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shape,
                DrawMode::Stroke(StrokeMode::new(Color::BLACK, 1.0)),
                Transform::default().with_translation(Vec3::new(position.x, position.y, 0.0)),
            ))
            .insert(Asteroid)
            .insert(Collidable)
            .insert(asteroid.0.clone())
            .insert(Velocity(velocity))
            .insert(AngularVelocity(rng.gen_range(-3.0..3.0)))
            .insert(BoundaryRemoval);
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

fn steering_control_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut AngularVelocity, &SteeringControl)>,
) {
    for (mut angular_velocity, steering) in query.iter_mut() {
        if keyboard_input.pressed(KeyCode::Left) {
            angular_velocity.0 = steering.0.get();
        } else if keyboard_input.pressed(KeyCode::Right) {
            angular_velocity.0 = -steering.0.get();
        } else {
            angular_velocity.0 = 0.0;
        }
    }
}

fn thrust_control_system(keyboard_input: Res<Input<KeyCode>>, mut query: Query<&mut ThrustEngine>) {
    for mut thrust_engine in query.iter_mut() {
        thrust_engine.on = keyboard_input.pressed(KeyCode::Up)
    }
}

fn weapon_control_system(keyboard_input: Res<Input<KeyCode>>, mut query: Query<&mut Weapon>) {
    for mut weapon in query.iter_mut() {
        weapon.triggered = weapon.triggered || keyboard_input.just_pressed(KeyCode::Space);
    }
}

fn explosion_system(mut query: Query<&mut Transform, With<Explosion>>) {
    for mut transform in query.iter_mut() {
        transform.scale.x += 0.001;
        transform.scale.y += 0.001;
        transform.scale.z += 0.001;
    }
}

fn ship_hit_system(
    mut rng: Local<Random>,
    mut ship_hits: EventReader<HitEvent<Asteroid, Ship>>,
    mut commands: Commands,
    query: Query<&Spatial, With<Ship>>,
) {
    for hit in ship_hits.iter() {
        let ship = hit.hurtable();

        if let Ok(spatial) = query.get(ship) {
            for n in 0..12 * 6 {
                let angle = 2.0 * PI / 12.0 * (n % 12) as f32 + rng.gen_range(0.0..2.0 * PI / 12.0);
                let direction = Vec2::new(angle.cos(), angle.sin());
                let position = direction * rng.gen_range(1.0..20.0) + spatial.position;

                commands
                    .entity(ship)
                    .insert(Ship::dead(Duration::from_secs(2)));

                commands
                    .spawn_bundle(ExplosionBundle::default())
                    .insert(Spatial {
                        position,
                        radius: 1.0,
                        ..spatial.clone()
                    })
                    .insert(Velocity(
                        Vec2::new(angle.cos(), angle.sin()) * rng.gen_range(150.0..250.0),
                    ))
                    .insert(Expiration::new(Duration::from_millis(
                        rng.gen_range(1000..1500),
                    )))
                    .insert(Flick::new(Duration::from_millis(rng.gen_range(20..30))));
            }
        }
    }
}

fn asteroid_hit_system(
    asteroid_sizes: Res<AsteroidSizes>,
    mut rng: Local<Random>,
    mut asteroid_hits: EventReader<HitEvent<Bullet, Asteroid>>,
    mut asteroid_spawn: EventWriter<AsteroidSpawnEvent>,
    mut commands: Commands,
    query: Query<&Spatial, With<Asteroid>>,
) {
    for hit in asteroid_hits.iter() {
        let asteroid = hit.hurtable();
        let bullet = hit.hittable();

        if let Ok(spatial) = query.get(asteroid) {
            if asteroid_sizes.big.contains(&spatial.radius) {
                let spatial = Spatial {
                    radius: rng.gen_range(asteroid_sizes.medium.clone()),
                    ..spatial.clone()
                };

                asteroid_spawn.send(AsteroidSpawnEvent(spatial.clone()));
                asteroid_spawn.send(AsteroidSpawnEvent(spatial.clone()));
                asteroid_spawn.send(AsteroidSpawnEvent(spatial.clone()));
            } else if asteroid_sizes.medium.contains(&spatial.radius) {
                let spatial = Spatial {
                    radius: rng.gen_range(asteroid_sizes.small.clone()),
                    ..spatial.clone()
                };
                asteroid_spawn.send(AsteroidSpawnEvent(spatial.clone()));
                asteroid_spawn.send(AsteroidSpawnEvent(spatial.clone()));
            }

            for n in 0..12 {
                let angle = 2.0 * PI / 12.0 * n as f32 + rng.gen_range(0.0..2.0 * PI / 12.0);
                let direction = Vec2::new(angle.cos(), angle.sin());
                let position = direction * rng.gen_range(1.0..20.0) + spatial.position;

                commands
                    .spawn_bundle(ExplosionBundle::default())
                    .insert(Spatial {
                        position,
                        radius: 1.0,
                        ..spatial.clone()
                    })
                    .insert(Velocity(
                        Vec2::new(angle.cos(), angle.sin()) * rng.gen_range(50.0..100.0),
                    ))
                    .insert(Expiration::new(Duration::from_millis(
                        rng.gen_range(400..700),
                    )))
                    .insert(Flick::new(Duration::from_millis(rng.gen_range(20..30))));
            }
        }

        commands.entity(asteroid).despawn();
        commands.entity(bullet).despawn();
    }
}
