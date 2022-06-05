#![allow(clippy::type_complexity)]

use std::{f32::consts::PI, ops::Range, time::Duration};

use bevy::{core::FixedTimestep, prelude::*, utils::HashSet, window::PresentMode};
use bevy_prototype_lyon::{
    entity::ShapeBundle,
    prelude::{
        tess::{geom::Rotation, math::Angle},
        *,
    },
    shapes::Polygon,
};
use boundary::{BoundaryPlugin, BoundaryRemoval, BoundaryWrap, Bounding};
use collision::{Collidable, CollisionPlugin, CollisionSystemLabel, HitEvent};
use expiration::{Expiration, ExpirationPlugin};
use flickering::{Flick, FlickPlugin};
use physics::{AngularVelocity, Damping, PhysicsPlugin, PhysicsSystemLabel, SpeedLimit, Velocity};
use rand::{prelude::SliceRandom, Rng};
use random::{Random, RandomPlugin};

mod boundary;
mod collision;
mod expiration;
mod flickering;
mod physics;
mod random;

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
        .insert_resource(AsteroidSizes {
            big: 50.0..60.0,
            medium: 30.0..40.0,
            small: 10.0..20.0,
        })
        .add_event::<AsteroidSpawnEvent>()
        .add_plugin(RandomPlugin)
        .add_plugin(PhysicsPlugin::with_fixed_time_step(1.0 / 120.0))
        .add_plugin(CollisionPlugin::<Bullet, Asteroid>::new())
        .add_plugin(CollisionPlugin::<Bullet, Ufo>::new())
        .add_plugin(CollisionPlugin::<Bullet, Ship>::new())
        .add_plugin(CollisionPlugin::<Asteroid, Ship>::new())
        .add_plugin(CollisionPlugin::<Asteroid, Ufo>::new())
        .add_plugin(CollisionPlugin::<Ufo, Ship>::new())
        .add_plugin(BoundaryPlugin)
        .add_plugin(ExpirationPlugin)
        .add_plugin(FlickPlugin)
        .add_startup_system(setup_system)
        .add_system_set(
            SystemSet::new()
                .label("input")
                .before(PhysicsSystemLabel)
                .with_system(steering_control_system)
                .with_system(thrust_control_system)
                .with_system(weapon_control_system),
        )
        .add_system(weapon_system.after("input"))
        .add_system(thrust_system.after("input"))
        .add_system(asteroid_spawn_system.with_run_criteria(FixedTimestep::step(0.5)))
        .add_system(asteroid_generation_system)
        .add_system(ufo_spawn_system.with_run_criteria(FixedTimestep::step(1.0)))
        .add_system(explosion_system)
        .add_system(ship_state_system.before(CollisionSystemLabel))
        .add_system(ufo_state_system.before(CollisionSystemLabel))
        .add_system(asteroid_hit_system.after(CollisionSystemLabel))
        .add_system(ship_hit_system.after(CollisionSystemLabel))
        .add_system(ufo_hit_system.after(CollisionSystemLabel))
        .run();
}

#[derive(Debug, Clone)]
struct AsteroidSizes {
    big: Range<f32>,
    medium: Range<f32>,
    small: Range<f32>,
}

#[derive(Debug, Component, Default)]
struct ThrustEngine {
    force: f32,
    on: bool,
}

impl ThrustEngine {
    fn new(force: f32) -> Self {
        Self {
            force,
            ..Default::default()
        }
    }
}

#[derive(Debug, Component, Default)]
struct SteeringControl(Angle);

#[derive(Debug, Component)]
struct Weapon {
    cooldown: Timer,
    force: f32,
    triggered: bool,
    automatic: bool,
}

impl Default for Weapon {
    fn default() -> Self {
        Self {
            cooldown: Timer::default(),
            force: 1000.0,
            triggered: false,
            automatic: false,
        }
    }
}

impl Weapon {
    fn new(rate_of_fire: Duration) -> Self {
        Self {
            cooldown: Timer::new(rate_of_fire, true),
            ..Default::default()
        }
    }
}

#[derive(Debug, Component, Deref)]
struct WeaponTarget(Entity);

#[derive(Debug, Component, Default)]
struct Ship {
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
struct Bullet;

#[derive(Debug, Component, Default)]
struct Ufo {
    state: UfoState,
}

#[derive(Debug)]
enum UfoState {
    Alive(Timer),
    ChangingDirection(Timer),
}

impl Default for UfoState {
    fn default() -> Self {
        UfoState::Alive(Timer::new(Duration::from_secs(5), false))
    }
}

impl Ufo {
    fn alive(duration: Duration) -> Self {
        Ufo {
            state: UfoState::Alive(Timer::new(duration, false)),
        }
    }

    fn changing_direction(duration: Duration) -> Self {
        Ufo {
            state: UfoState::ChangingDirection(Timer::new(duration, false)),
        }
    }
}

#[derive(Debug, Component, Default)]
struct Explosion;

#[derive(Debug, Component, Default)]
struct Asteroid;

#[derive(Debug)]
struct AsteroidSpawnEvent(Vec2, Bounding);

#[derive(Bundle)]
struct ExplosionBundle {
    #[bundle]
    shape_bundle: ShapeBundle,
    explosion: Explosion,
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
            velocity: Velocity::default(),
            damping: Damping::from(0.97),
            expiration: Expiration::new(Duration::from_secs(1)),
        }
    }
}

fn setup_system(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn().insert(Ship::spawn(Duration::from_secs(0)));
}

fn thrust_system(mut query: Query<(&mut Velocity, &ThrustEngine, &Transform)>) {
    for (mut velocity, thrust, transform) in query.iter_mut() {
        if thrust.on {
            let dir = transform.rotation * Vec3::X;
            velocity.x += dir.x * thrust.force;
            velocity.y += dir.y * thrust.force;
        }
    }
}

fn weapon_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(&Bounding, Option<&WeaponTarget>, &Transform, &mut Weapon)>,
    targets: Query<&Transform>,
) {
    for (bounds, target, transform, mut weapon) in query.iter_mut() {
        weapon.cooldown.tick(time.delta());

        if weapon.cooldown.finished() && weapon.triggered {
            weapon.triggered = false;

            let bullet_dir = match target.and_then(|target| targets.get(**target).ok()) {
                Some(target) => (target.translation - transform.translation).normalize_or_zero(),
                None => transform.rotation * Vec3::X,
            };

            let bullet_vel = bullet_dir * weapon.force;
            let bounds = **bounds + 10.0;
            let bullet_pos = transform.translation + (bullet_dir * bounds);

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
                .insert(Bounding::from_radius(2.0))
                .insert(Velocity::from(Vec2::new(bullet_vel.x, bullet_vel.y)))
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
                        .insert(Bounding::from_radius(12.0))
                        .insert(Velocity::default())
                        .insert(SpeedLimit::from(350.0))
                        .insert(Damping::from(0.998))
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

fn ufo_state_system(
    time: Res<Time>,
    mut rng: Local<Random>,
    mut commands: Commands,
    mut ufos: Query<(Entity, &mut Weapon, &mut Ufo, &mut Velocity, &Transform), With<Ufo>>,
    ships: Query<(Entity, &Ship), With<Ship>>,
) {
    let alive_ships = ships
        .iter()
        .filter_map(|(entity, ship)| match ship.state {
            ShipState::Alive => Some(entity),
            _ => None,
        })
        .collect::<Vec<_>>();

    for (entity, mut weapon, mut ufo, mut velocity, transform) in ufos.iter_mut() {
        match ufo.state {
            UfoState::Alive(ref mut timer) => {
                timer.tick(time.delta());

                if timer.finished() {
                    *ufo =
                        Ufo::changing_direction(Duration::from_millis(rng.gen_range(1000..2000)));
                }
            }

            UfoState::ChangingDirection(ref mut timer) => {
                if timer.elapsed().is_zero() {
                    velocity.y = if transform.translation.y > 0.0 {
                        -100.0
                    } else {
                        100.0
                    };
                }

                timer.tick(time.delta());

                if timer.finished() {
                    velocity.y = 0.0;
                    *ufo = Ufo::alive(Duration::from_secs(rng.gen_range(4..8)));
                }
            }
        }

        if let Some(ship) = alive_ships.choose(&mut **rng) {
            weapon.triggered = true;
            commands.entity(entity).insert(WeaponTarget(*ship));
        } else {
            weapon.triggered = false;
            commands.entity(entity).remove::<WeaponTarget>();
        }
    }
}

fn ufo_spawn_system(
    window: Res<WindowDescriptor>,
    mut rng: Local<Random>,
    mut commands: Commands,
    ships: Query<Entity, With<Ship>>,
) {
    if rng.gen_bool(1.0 / 10.0) {
        let h = (window.height * 0.8) / 2.0;
        let w = window.width / 2.0;

        let y = rng.gen_range(-h..h);
        let x = [-w, w].choose(&mut **rng).copied().unwrap();

        let c = 30.0;
        let position = Vec3::new(if x > 0.0 { w + c } else { -w - c }, y, 0.0);

        let mut ufo = commands.spawn();

        ufo.insert_bundle(GeometryBuilder::build_as(
            &shapes::Rectangle {
                extents: Vec2::new(c, c / 2.0),
                ..Default::default()
            },
            DrawMode::Stroke(StrokeMode::new(Color::BLACK, 1.0)),
            Transform::default().with_translation(position),
        ))
        .insert_bundle(GeometryBuilder::build_as(
            &{
                let h = c / 2.5;
                let w = c;
                let hw = w / 2.0;
                let hh = h / 2.0;

                let mut path_builder = PathBuilder::new();
                path_builder.move_to(Vec2::new(0.0, -hh));
                path_builder.line_to(Vec2::new(-hw * 0.7, -hh));
                path_builder.line_to(Vec2::new(-hw, 0.0));
                path_builder.line_to(Vec2::new(-hw * 0.7, hh));
                path_builder.line_to(Vec2::new(hw * 0.7, hh));
                path_builder.line_to(Vec2::new(hw, 0.0));
                path_builder.line_to(Vec2::new(hw * 0.7, -hh));
                path_builder.line_to(Vec2::new(0.0, -hh));

                path_builder.move_to(Vec2::new(-hw, 0.0));
                path_builder.line_to(Vec2::new(hw, 0.0));

                path_builder.move_to(Vec2::new(-hw * 0.5, hh));
                path_builder.line_to(Vec2::new(-hw * 0.3, hh * 1.8));
                path_builder.line_to(Vec2::new(hw * 0.3, hh * 1.8));
                path_builder.line_to(Vec2::new(hw * 0.5, hh));

                path_builder.build()
            },
            DrawMode::Stroke(StrokeMode::new(Color::BLACK, 1.0)),
            Transform::default().with_translation(position),
        ))
        .insert(Ufo::alive(Duration::from_secs(rng.gen_range(1..5))))
        .insert(Weapon {
            force: rng.gen_range(300.0..500.0),
            triggered: true,
            automatic: true,
            ..Weapon::new(Duration::from_millis(rng.gen_range(1000..3000)))
        })
        .insert(Bounding::from_radius(c / 2.0))
        .insert(Velocity::from(
            Vec2::new(if x > 0.0 { -1.0 } else { 1.0 }, 0.0) * rng.gen_range(100.0..200.0),
        ))
        .insert(Collidable)
        .insert(BoundaryRemoval);

        if let Some(ship) = ships.iter().collect::<Vec<_>>().choose(&mut **rng) {
            ufo.insert(WeaponTarget(*ship));
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

        asteroids.send(AsteroidSpawnEvent(position, Bounding::from_radius(radius)));
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

    for AsteroidSpawnEvent(position, bounds) in asteroids.iter() {
        let velocity = Vec2::new(rng.gen_range(-w..w), rng.gen_range(-h..h));
        let scale = if asteroid_sizes.big.contains(bounds) {
            rng.gen_range(30.0..60.0)
        } else if asteroid_sizes.medium.contains(bounds) {
            rng.gen_range(60.0..80.0)
        } else {
            rng.gen_range(80.0..100.0)
        };
        let velocity = (velocity - *position).normalize_or_zero() * scale;

        let shape = {
            let sides = rng.gen_range(6..12);
            let mut points = Vec::with_capacity(sides);
            let n = sides as f32;
            let internal = (n - 2.0) * PI / n;
            let offset = -internal / 2.0;
            let step = 2.0 * PI / n;
            let r = **bounds;
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
            .insert(*bounds)
            .insert(Velocity::from(velocity))
            .insert(AngularVelocity::from(rng.gen_range(-3.0..3.0)))
            .insert(BoundaryRemoval);
    }
}

fn steering_control_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut AngularVelocity, &SteeringControl)>,
) {
    for (mut angular_velocity, steering) in query.iter_mut() {
        if keyboard_input.pressed(KeyCode::Left) {
            *angular_velocity = AngularVelocity::from(steering.0.get());
        } else if keyboard_input.pressed(KeyCode::Right) {
            *angular_velocity = AngularVelocity::from(-steering.0.get());
        } else {
            *angular_velocity = AngularVelocity::from(0.0);
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
        let pressed = if weapon.automatic {
            keyboard_input.pressed(KeyCode::Space)
        } else {
            keyboard_input.just_pressed(KeyCode::Space)
        };
        weapon.triggered = weapon.triggered || pressed;
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
    mut asteroid_hits: EventReader<HitEvent<Asteroid, Ship>>,
    mut bullet_hits: EventReader<HitEvent<Bullet, Ship>>,
    mut ufo_hits: EventReader<HitEvent<Ufo, Ship>>,
    mut commands: Commands,
    query: Query<&Transform, With<Ship>>,
) {
    let hits = asteroid_hits
        .iter()
        .map(|hit| hit.hurtable())
        .chain(bullet_hits.iter().map(|hit| hit.hurtable()))
        .chain(ufo_hits.iter().map(|hit| hit.hurtable()));

    for ship in hits {
        if let Ok(transform) = query.get(ship) {
            for n in 0..12 * 6 {
                let angle = 2.0 * PI / 12.0 * (n % 12) as f32 + rng.gen_range(0.0..2.0 * PI / 12.0);
                let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                let position = direction * rng.gen_range(1.0..20.0) + transform.translation;

                commands
                    .entity(ship)
                    .insert(Ship::dead(Duration::from_secs(2)));

                commands
                    .spawn_bundle(ExplosionBundle::default())
                    .insert(Transform::default().with_translation(position))
                    .insert(Velocity::from(
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
    query: Query<(&Transform, &Bounding), With<Asteroid>>,
) {
    let mut removed = HashSet::with_capacity(asteroid_hits.len());

    for hit in asteroid_hits.iter() {
        let asteroid = hit.hurtable();
        let bullet = hit.hittable();

        if removed.contains(&asteroid) || removed.contains(&bullet) {
            continue;
        }

        if let Ok((transform, radius)) = query.get(asteroid) {
            let position = Vec2::new(transform.translation.x, transform.translation.y);

            let explosion_size = if asteroid_sizes.big.contains(radius) {
                let bounds = Bounding::from_radius(rng.gen_range(asteroid_sizes.medium.clone()));
                asteroid_spawn.send(AsteroidSpawnEvent(position, bounds));
                asteroid_spawn.send(AsteroidSpawnEvent(position, bounds));
                5
            } else if asteroid_sizes.medium.contains(radius) {
                let bounds = Bounding::from_radius(rng.gen_range(asteroid_sizes.small.clone()));
                asteroid_spawn.send(AsteroidSpawnEvent(position, bounds));
                asteroid_spawn.send(AsteroidSpawnEvent(position, bounds));
                asteroid_spawn.send(AsteroidSpawnEvent(position, bounds));
                3
            } else {
                1
            };

            for n in 0..12 * explosion_size {
                let angle = 2.0 * PI / 12.0 * (n % 12) as f32 + rng.gen_range(0.0..2.0 * PI / 12.0);
                let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                let position = direction * rng.gen_range(1.0..20.0) + transform.translation;

                commands
                    .spawn_bundle(ExplosionBundle::default())
                    .insert(Transform::default().with_translation(position))
                    .insert(Velocity::from(
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

        removed.insert(asteroid);
        removed.insert(bullet);
    }
}

fn ufo_hit_system(
    mut rng: Local<Random>,
    mut bullet_hits: EventReader<HitEvent<Bullet, Ufo>>,
    mut asteroid_hits: EventReader<HitEvent<Asteroid, Ufo>>,
    mut commands: Commands,
    query: Query<&Transform, With<Ufo>>,
) {
    let mut removed = HashSet::with_capacity(bullet_hits.len() + asteroid_hits.len());

    let hits = bullet_hits
        .iter()
        .map(|hit| hit.hurtable())
        .chain(asteroid_hits.iter().map(|hit| hit.hurtable()));

    for ufo in hits {
        if removed.contains(&ufo) {
            continue;
        }

        if let Ok(transform) = query.get(ufo) {
            for n in 0..12 * 5 {
                let angle = 2.0 * PI / 12.0 * (n % 12) as f32 + rng.gen_range(0.0..2.0 * PI / 12.0);
                let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                let position = direction * rng.gen_range(1.0..20.0) + transform.translation;

                commands
                    .spawn_bundle(ExplosionBundle::default())
                    .insert(Transform::default().with_translation(position))
                    .insert(Velocity::from(
                        Vec2::new(angle.cos(), angle.sin()) * rng.gen_range(50.0..100.0),
                    ))
                    .insert(Expiration::new(Duration::from_millis(
                        rng.gen_range(400..700),
                    )))
                    .insert(Flick::new(Duration::from_millis(rng.gen_range(20..30))));
            }
        }

        commands.entity(ufo).despawn();
        removed.insert(ufo);
    }

    for hit in bullet_hits.iter() {
        let bullet = hit.hittable();
        if removed.contains(&bullet) {
            continue;
        }
        commands.entity(bullet).despawn();
        removed.insert(bullet);
    }
}
