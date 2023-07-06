use bevy::{prelude::*, math::Vec3Swizzles};

fn main() {
    let mut app = App::new();
    app
        .register_type::<Velocity>()
        .register_type::<Wind>()
        .register_type::<Object>()
        .register_type::<Sail>()
        ;

    app
        .add_plugins(DefaultPlugins)
        .add_startup_system(sys_setup)
        .add_systems((sys_wind_physics, sys_apply_velocity).chain())
        .insert_resource(Wind(Vec2::new(0.0, 30.0)))
        ;


    #[cfg(feature = "debug")]
    {
        app.add_plugin(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
    }

    app.run();
}

fn sys_setup(
    mut commands: Commands,
) {
    commands.spawn(Camera2dBundle::default());

    spawn_ship(commands);
}


fn spawn_ship(mut cmd: Commands) {
    cmd.spawn(Name::new("Ship"))
        .insert(Object)
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::MAROON,
                custom_size: Some(Vec2::new(35.0, 80.0)),
                ..default()
            },
            ..default()
        })
        .insert(Velocity(Vec2::new(0.0, 0.0)))
        .with_children(|ship| {
            ship.spawn(Name::new("Sail"))
                .insert((Object, Sail { drag_coefficient: 0.3 }))
                .insert(SpriteBundle {
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(75.0, 10.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 20.0, 1.0),
                    ..default()
                });
        });
}


#[derive(Default, Copy, Clone, Component, Reflect)]
struct Velocity(Vec2);

fn sys_apply_velocity(time: Res<Time>, mut q: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in q.iter_mut() {
        transform.translation += time.delta_seconds() * velocity.0.extend(0.0);
    }
}

#[derive(Default, Copy, Clone, Component, Reflect)]
struct Object;

#[derive(Default, Copy, Clone, Component, Reflect)]
struct Sail {
    drag_coefficient: f32,
}

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
struct Wind(Vec2);



fn sys_wind_physics(
    time: Res<Time>,
    wind: Res<Wind>,
    mut q_boat: Query<(&mut Velocity, &Transform), With<Object>>,
    q_sail: Query<(&Sail, &GlobalTransform, &Parent)>,
) {

    const BOAT_MASS: f32 = 1.0;

    let real_wind_v = wind.0;
    let dt = time.delta_seconds();

    for (sail, sail_xf, sail_parent) in q_sail.iter() {
        let Ok((mut boat_velocity, &boat_xf)) = q_boat.get_mut(sail_parent.get()) else { continue };
        let boat_v = boat_velocity.0;
        let sail_dir = sail_xf.up().xy();
        let boat_dir = boat_xf.up().xy();
        let drag = sail.drag_coefficient;

        let apparent_wind_v = real_wind_v - boat_v;
        let sail_f = drag * apparent_wind_v.project_onto(sail_dir);
        let boat_f = sail_f.project_onto(boat_dir);
        let boat_a = boat_f / BOAT_MASS;

        // dbg!(real_wind_v, dt, boat_v, sail_dir, boat_dir, drag, apparent_wind_v, sail_f, boat_f, boat_a);

        boat_velocity.0 += boat_a * dt;

        //panic!("bailing");
    }
}