use bevy::ecs::system::Despawn;
use bevy::prelude::*;
use heron::prelude::*;
use bevy::input::system::exit_on_esc_system;
use bevy::sprite::collide_aabb::collide;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .add_system(exit_on_esc_system)
        .add_startup_system(setup)
        .add_system(enemy_movement)
        .add_system(player_movement)
        .add_system(shoot)
        .add_system(expire_projectile)

        // this works?
        // .add_system(projectile_collision_check)

        // these have no physics effect on the enemy (except sometimes?)
        // .add_system(projectile_collision_event)
        // .add_system(projectile_collide_check)

        // these have an effect, but don't despawn correctly
        .add_system_to_stage(CoreStage::PostUpdate, projectile_collision_event)
        // .add_system_to_stage(CoreStage::PostUpdate, projectile_collide_check)

        .run();
}


#[derive(PhysicsLayer)]
enum Layer {
    Enemy,
    Player,
    Projectile,
}

impl Layer {
    pub fn enemy() -> CollisionLayers {
        CollisionLayers::none()
            .with_group(Self::Enemy)
            .with_masks(&[Self::Player, Self::Enemy, Self::Projectile])
    }
    pub fn player() -> CollisionLayers {
        CollisionLayers::none()
            .with_group(Self::Player)
            .with_mask(Self::Enemy)
    }
    pub fn projectile() -> CollisionLayers {
        CollisionLayers::none()
            .with_group(Self::Projectile)
            .with_mask(Self::Enemy)
    }
}

#[derive(Component)]
struct Player {
    cooldown: Timer,
}

#[derive(Component)]
struct Projectile {
    expire_in: f32,
}

#[derive(Component)]
struct Enemy;

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::GREEN,
                ..Default::default()
            },
            transform: Transform::from_scale(Vec3::splat(40.)),
            ..Default::default()
        }).insert(Player {
            cooldown: Timer::from_seconds(1., true),
        })
        .insert(RigidBody::KinematicPositionBased)
        .insert(Layer::player())
        .insert(CollisionShape::Cuboid {
            half_extends: Vec3::splat(20.),
            border_radius: None,
        });

    commands.spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::RED,
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(600., 0., 0.))
                .with_scale(Vec3::splat(40.)),
            ..Default::default()
        }).insert(Enemy)
        .insert(RigidBody::Dynamic)
        .insert(Layer::enemy())
        .insert(Velocity::default())
        .insert(CollisionShape::Cuboid {
            half_extends: Vec3::splat(20.),
            border_radius: None,
        });
}

fn projectile_collision_event(
    mut commands: Commands,
    mut events: EventReader<CollisionEvent>,
) {
    for event in events.iter() {
        match event {
            CollisionEvent::Started(data1, data2) => {
                info!("Entity {:?} and {:?} started to collide", data1.rigid_body_entity(), data2.rigid_body_entity());
                if data1.collision_layers().contains_group(Layer::Projectile) {
                    commands.add(Despawn {
                        entity: data1.rigid_body_entity(),
                    });
                    // commands.entity(data1.rigid_body_entity()).despawn(); // crashes sometimes
                }
                else if data2.collision_layers().contains_group(Layer::Projectile) {
                    commands.add(Despawn {
                        entity: data2.rigid_body_entity(),
                    });
                    // commands.entity(data2.rigid_body_entity()).despawn();
                }
            }
            CollisionEvent::Stopped(data1, data2) => {
                info!("Entity {:?} and {:?} stopped to collide", data1.rigid_body_entity(), data2.rigid_body_entity());
            }
        }
    }
}

fn projectile_collision_check(
    mut commands: Commands,
    projectile_query: Query<(Entity, &Collisions), With<Projectile>>,
) {
    for (projectile_entity, projectile_collisions) in projectile_query.iter() {
        if let Some(enemy) = projectile_collisions.entities().next() {
            info!("Projectile {:?} hit enemy {:?}", projectile_entity.id(), enemy.id());
            commands.entity(projectile_entity).despawn();
        }
    }
}

fn projectile_collide_check(
    mut commands: Commands,
    enemy_query: Query<&Transform, With<Enemy>>,
    projectile_query: Query<(Entity, &Transform), With<Projectile>>,
) {
    'projectile_loop: for (projectile_entity, projectile_transform) in projectile_query.iter() {
        for enemy_transform in enemy_query.iter() {
            let collision = collide(
                projectile_transform.translation,
                projectile_transform.scale.truncate(),
                enemy_transform.translation,
                enemy_transform.scale.truncate(),
            );
            if collision.is_some() {
                info!("Projectile {:?} hit enemy", projectile_entity.id());
                commands.entity(projectile_entity).despawn();
                continue 'projectile_loop
            }
        }
    }
}

fn enemy_movement(
    mut query: Query<(&Transform, &mut Velocity), With<Enemy>>,
    player_query: Query<&Transform, With<Player>>,
) {
    for (transform, mut velocity) in query.iter_mut() {
        for player_transform in player_query.iter() {
            if let Some(direction) = (player_transform.translation - transform.translation).try_normalize() {
                *velocity = Velocity::from_linear(direction * 100.);
            }
        }
    }
}

fn shoot(
    mut commands: Commands,
    time: Res<Time>,
    enemy_query: Query<&Transform, With<Enemy>>,
    mut player_query: Query<(&Transform, &mut Player)>,
) {
    for (player_transform, mut player) in player_query.iter_mut() {
        if player.cooldown.tick(time.delta()).just_finished() {
            let enemy_transform = enemy_query.get_single().unwrap();
            let direction = (enemy_transform.translation - player_transform.translation).normalize();
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::BLUE,
                        ..Default::default()
                    },
                    transform: Transform::from_translation(player_transform.translation)
                        .with_scale(Vec3::splat(5.)),
                    ..Default::default()
                })
                .insert(Projectile {
                    expire_in: 1.,
                })
                .insert(RigidBody::KinematicVelocityBased)
                .insert(Layer::projectile())
                .insert(CollisionShape::Sphere {
                    radius: 2.5,
                })
                .insert(Velocity::from_linear(direction * 300.))
                .insert(Collisions::default());
        }
    }
}

fn expire_projectile(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Projectile)>,
) {
    for (entity, mut projectile) in query.iter_mut() {
        projectile.expire_in -= time.delta_seconds();
        if projectile.expire_in <= 0. {
            commands.entity(entity).despawn();
        }
    }
}

fn player_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let mut dir = Vec3::default();
    if keyboard_input.pressed(KeyCode::W) {
        dir.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) {
        dir.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::A) {
        dir.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::D) {
        dir.x += 1.0;
    }
    if let Some(direction) = dir.try_normalize() {
        for mut transform in query.iter_mut() {
            transform.translation += direction * 250. * time.delta_seconds()
        }
    }
}