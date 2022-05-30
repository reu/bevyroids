use std::{f32::consts::PI, time::Duration};

use bevy::{core::FixedTimestep, prelude::*, window::PresentMode};
use bevy_prototype_lyon::{
    prelude::{
        tess::{geom::Rotation, math::Angle},
        *,
    },
    shapes::Polygon,
};
use rand::{prelude::SmallRng, Rng, SeedableRng};

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
        .add_system(drawing_system.after("wrap"))
        .run();
}

fn setup_system(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands
        .spawn_bundle(GeometryBuilder::build_as(
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
        .insert(Weapon::new(Duration::from_millis(100)))
        .insert(BoundaryWrap);
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

#[derive(Debug, Component, Default)]
struct Spatial {
    position: Vec2,
    rotation: f32,
    radius: f32,
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

#[derive(Debug, Component, Default)]
struct BoundaryWrap;

#[derive(Debug, Component, Default)]
struct BoundaryRemoval;

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

fn asteroid_spawn_system(
    window: Res<WindowDescriptor>,
    mut rng: Local<Random>,
    mut commands: Commands,
) {
    if rng.gen_bool(1.0 / 3.0) {
        let w = window.width / 2.0;
        let h = window.height / 2.0;

        let x = rng.gen_range(-w..w);
        let y = rng.gen_range(-h..h);
        let r = rng.gen_range(30.0..40.0);
        let c = r * 2.0;

        let position = if rng.gen_bool(1.0 / 2.0) {
            Vec2::new(x, if y > 0.0 { h + c } else { -h - c })
        } else {
            Vec2::new(if x > 0.0 { w + c } else { -w - c }, y)
        };

        let velocity = Vec2::new(rng.gen_range(-w..w), rng.gen_range(-h..h));
        let velocity = (velocity - position).normalize_or_zero() * rng.gen_range(30.0..60.0);

        let shape = {
            let sides = rng.gen_range(6..12);
            let mut points = Vec::with_capacity(sides);
            let n = sides as f32;
            let internal = (n - 2.0) * PI / n;
            let offset = -internal / 2.0;
            let step = 2.0 * PI / n;
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
            .insert(Spatial {
                position,
                rotation: 0.0,
                radius: r,
            })
            .insert(Velocity(velocity))
            .insert(AngularVelocity(rng.gen_range(-3.0..3.0)))
            .insert(BoundaryRemoval);
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

fn drawing_system(mut query: Query<(&mut Transform, &Spatial)>) {
    for (mut transform, spatial) in query.iter_mut() {
        transform.translation.x = spatial.position.x;
        transform.translation.y = spatial.position.y;
        transform.rotation = Quat::from_rotation_z(spatial.rotation);
    }
}
