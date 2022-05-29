use bevy::{core::FixedTimestep, prelude::*, window::PresentMode};
use bevy_prototype_lyon::prelude::{
    tess::{geom::Rotation, math::Angle},
    *,
};

const TIME_STEP: f32 = 1.0 / 60.0;

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
        .add_startup_system(setup_system)
        .add_system_set(
            SystemSet::new()
                .label("input")
                .with_system(ship_control_system)
                .with_system(thrust_control_system)
                .with_system(weapon_control_system),
        )
        .add_system(weapon_system.after("input").before("physics"))
        .add_system(thrust_system.after("input").before("physics"))
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP.into()))
                .label("physics")
                .after("input")
                .with_system(damping_system)
                .with_system(movement_system)
                .with_system(weapon_system),
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
        .insert(SpeedLimit(400.0))
        .insert(Damping(0.988))
        .insert(ThrustEngine::default())
        .insert(Weapon {
            rate_of_fire: 9.0,
            ..Default::default()
        })
        .insert(BoundaryWrap)
        .insert(Ship);
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
struct SpeedLimit(f32);

#[derive(Debug, Component, Default)]
struct Damping(f32);

#[derive(Debug, Component, Default)]
struct ThrustEngine(bool);

#[derive(Debug, Component, Default)]
struct Weapon {
    rate_of_fire: f32,
    triggered: bool,
}

#[derive(Debug, Component, Default)]
struct WeaponCooldown(f32);

#[derive(Debug, Component, Default)]
struct Ship;

#[derive(Debug, Component, Default)]
struct BoundaryWrap;

#[derive(Debug, Component, Default)]
struct BoundaryRemoval;

fn movement_system(mut query: Query<(&mut Spatial, &Velocity, Option<&SpeedLimit>)>) {
    for (mut spatial, velocity, speed_limit) in query.iter_mut() {
        spatial.position += match speed_limit {
            Some(SpeedLimit(limit)) => velocity.0.clamp_length_max(*limit),
            None => velocity.0,
        } * TIME_STEP;
    }
}

fn damping_system(mut query: Query<(&mut Velocity, &Damping)>) {
    for (mut velocity, damping) in query.iter_mut() {
        velocity.0 *= damping.0;
    }
}

fn thrust_system(mut query: Query<(&mut Velocity, &ThrustEngine, &Spatial)>) {
    for (mut velocity, thrust, spatial) in query.iter_mut() {
        if thrust.0 {
            velocity.0.x += spatial.rotation.cos() * 2.0;
            velocity.0.y += spatial.rotation.sin() * 2.0;
        }
    }
}

fn weapon_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Spatial, &Weapon, Option<&mut WeaponCooldown>)>,
) {
    for (entity, spatial, weapon, mut cooldown) in query.iter_mut() {
        match cooldown {
            Some(ref mut cooldown) if cooldown.0 > 0.0 => {
                cooldown.0 -= 1.0;
            }
            Some(ref cooldown) if cooldown.0 <= 0.0 => {
                commands.entity(entity).remove::<WeaponCooldown>();
            }
            None if weapon.triggered => {
                commands
                    .entity(entity)
                    .insert(WeaponCooldown(weapon.rate_of_fire));

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
            _ => {}
        };
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

fn ship_control_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Spatial, With<Ship>>,
) {
    for mut spatial in query.iter_mut() {
        if keyboard_input.pressed(KeyCode::Left) {
            spatial.rotation += Angle::degrees(90.0).get() * TIME_STEP;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            spatial.rotation -= Angle::degrees(90.0).get() * TIME_STEP;
        }
    }
}

fn thrust_control_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut ThrustEngine, With<Ship>>,
) {
    for mut thrust_engine in query.iter_mut() {
        thrust_engine.0 = keyboard_input.pressed(KeyCode::Up)
    }
}

fn weapon_control_system(keyboard_input: Res<Input<KeyCode>>, mut query: Query<&mut Weapon>) {
    for mut weapon in query.iter_mut() {
        weapon.triggered = keyboard_input.pressed(KeyCode::Space)
    }
}

fn drawing_system(mut query: Query<(&mut Transform, &Spatial)>) {
    for (mut transform, spatial) in query.iter_mut() {
        transform.translation.x = spatial.position.x;
        transform.translation.y = spatial.position.y;
        transform.rotation = Quat::from_rotation_z(spatial.rotation);
    }
}
