use std::f32::consts::PI;

use bevy::{prelude::*, math::Vec3Swizzles, window::PrimaryWindow};
use bevy::color::palettes::basic;
use bevy_inspector_egui::bevy_egui::EguiPlugin;

fn main() {
    let mut app = App::new();
    app
        .register_type::<DebugGizmos>()
        .register_type::<Velocity>()
        .register_type::<Wind>()
        .register_type::<Object>()
        .register_type::<Sail>()
        .register_type::<InitialTransform>()
        .register_type::<EventResetTransform>()
        .register_type::<MousePos>()
        .register_type::<TurnRadius>()
        .register_type::<Constants>()
    ;


    app
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sailing Simulator?!".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, sys_setup)
        .add_systems(Startup, sys_spawn_ship)
        .add_systems(Update, sys_input)
        .add_systems(Update, sys_draw_debug_gizmos)
        .add_systems(Update, (sys_wind_physics, sys_apply_velocity).chain())
        .add_systems(Update, sys_circular_motion)
        .add_systems(Update, sys_friction_physics)
        .add_systems(Update, sys_reset_xf)
        .add_systems(Update, sys_mouse_track)
        .insert_resource(Wind(Vec2::new(0.0, 30.0)))
        .insert_resource(DebugGizmos(Vec2::new(300.0, -200.0)))
        .insert_resource(Constants::default())
        .init_resource::<MousePos>()
        .add_event::<EventResetTransform>()
    ;




    #[cfg(feature = "debug")]
    {
        app
            .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
            .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        ;
    }

    app.run();
}

fn sys_setup(
    mut commands: Commands,
) {
    commands.spawn(Camera2d::default());
}


fn sys_spawn_ship(mut cmd: Commands, mut meshes: ResMut<Assets<Mesh>>, mut colors: ResMut<Assets<ColorMaterial>>) {

    let sail_mesh = meshes.add(Rectangle::new(75.0, 10.0));
    let sail_color = colors.add(Color::WHITE);

    let ship_mesh = meshes.add(Rectangle::new(30.0, 80.0));
    let ship_color = colors.add(Color::Srgba(basic::MAROON));

    cmd.spawn(Name::new("Ship"))
        .insert(Object)
        .insert(InitialTransform(default()))
        .insert((Mesh2d(ship_mesh), MeshMaterial2d(ship_color)))
        .insert(Velocity(Vec2::new(0.0, 0.0)))
        .insert(TurnRadius(200.))
        .with_children(|ship| {
            ship.spawn(Name::new("Sail"))
                .insert((Object, Sail { drag_coefficient: 0.3 }))
                .insert((Mesh2d(sail_mesh), MeshMaterial2d(sail_color)))
                .insert(Transform::from_xyz(0., 15., 1.));
        });
}

#[derive(Default, Copy, Clone, Resource, Reflect)]
#[reflect(Resource)]
struct DebugGizmos(Vec2);


fn sys_draw_debug_gizmos(mut gizmos: Gizmos, debug_gizmos: Res<DebugGizmos>, wind: Res<Wind>) {
    const SCALE_FACTOR: f32 = 1.0;
    gizmos.circle_2d(debug_gizmos.0, 5.0, basic::RED);
    gizmos.line_2d(debug_gizmos.0, debug_gizmos.0 + SCALE_FACTOR * wind.0, basic::GREEN);
}

#[derive(Default, Copy, Clone, Component, Reflect)]
struct Velocity(Vec2);

fn sys_apply_velocity(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &Velocity)>,
) {
    for (mut transform, velocity) in q.iter_mut() {
        transform.translation += time.delta_secs() * velocity.0.extend(0.0);
    }
}

#[derive(Default, Copy, Clone, Component, Reflect)]
struct Object;

#[derive(Default, Copy, Clone, Component, Reflect)]
struct Sail {
    drag_coefficient: f32,
}

#[derive(Default, Copy, Clone, Component, Reflect)]
struct TurnRadius(f32);

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
struct Wind(Vec2);


fn sys_wind_physics(
    time: Res<Time>,
    wind: Res<Wind>,
    constants: Res<Constants>,
    mut q_boat: Query<(&mut Velocity, &Transform), With<Object>>,
    q_sail: Query<(&Sail, &GlobalTransform, &ChildOf)>,
) {

    let real_wind_v = wind.0;
    let dt = time.delta_secs();

    for (sail, sail_xf, sail_parent) in q_sail.iter() {
        let Ok((mut boat_velocity, &boat_xf)) = q_boat.get_mut(sail_parent.parent()) else { continue };
        let boat_v = boat_velocity.0;
        let sail_dir = sail_xf.up().xy();
        let boat_dir = boat_xf.up().xy();
        let drag = sail.drag_coefficient;

        let apparent_wind_v = real_wind_v - boat_v;
        let sail_f = drag * apparent_wind_v.project_onto(sail_dir);
        let boat_f = sail_f.project_onto(boat_dir);
        let boat_a = boat_f / constants.boat_mass;

        boat_velocity.0 += boat_a * dt;
    }
}

fn sys_circular_motion(
    time: Res<Time>,
    q_object: Query<(&TurnRadius, &mut Velocity, &mut Transform)>
) {
    for (&TurnRadius(rad), mut v, mut xf) in q_object {
        let dist = (v.0 * time.delta_secs()).length();
        let dtheta = dist / rad * 2. * PI;
        xf.rotate_z(dtheta);
        v.0 = v.0.rotate(Vec2::from_angle(dtheta));
    }

}

fn sys_friction_physics(
    time: Res<Time>,
    constants: Res<Constants>,
    mut q_object: Query<&mut Velocity, With<Object>>,
) {
    for mut v in &mut q_object {
        // |F| = c * |v|^m,
        // F = |F| * -v/|v|,
        // so F = -c * |v|^(m-1) * v.
        // Use m = 2 for now
        let f = -constants.boat_friction_coefficient * v.0.length() * v.0;
        v.0 = v.0 + f / constants.boat_mass * time.delta_secs();
    }
}

#[derive(Reflect, Default, Component)]
struct InitialTransform(Transform);

#[derive(Reflect, Default, Event)]
struct EventResetTransform;

fn sys_reset_xf(mut ev_reset_xf: EventReader<EventResetTransform>,
                mut q_transform: Query<(&mut Transform, Option<&mut Velocity>, &InitialTransform)>) {
    if ev_reset_xf.read().next().is_some() {
        for (mut xf, velocity, init_xf) in q_transform.iter_mut() {
            *xf = init_xf.0;
            if let Some(mut v) = velocity {
                *v = Velocity(Vec2::ZERO);
            }
        }
    }
}


#[derive(Reflect, Resource)]
#[reflect(Resource)]
struct Constants {
    sail_secs_per_rev: f32,
    wind_change_speed: f32,
    boat_turn_radius: f32,
    boat_friction_coefficient: f32,
    boat_mass: f32, // TODO make this part of Object
}

impl Default for Constants {
    fn default() -> Self {
        Self {
            sail_secs_per_rev: 3.,
            wind_change_speed: 50.,
            boat_turn_radius: 400.,
            boat_friction_coefficient: 0.01,
            boat_mass: 1.,
        }
    }
}


// TODO split this into separate systems for parallel processing
fn sys_input(keys: Res<ButtonInput<KeyCode>>,
             mut evw_reset_xf: EventWriter<EventResetTransform>,
             mut q_sail: Query<&mut Transform, With<Sail>>,
             mut q_turn: Query<&mut TurnRadius>,
             time: Res<Time>,
             constants: Res<Constants>,
             mut wind: ResMut<Wind>) {

    // RESET TRANSFORM //
    if keys.just_released(KeyCode::KeyR) {
        evw_reset_xf.write(EventResetTransform);
    }

    // SAIL ROTATION //
    let mut sail_needs_update = false;
    let mut sail_rot = 0.;

    let rotate_speed: f32 = PI * 2. / constants.sail_secs_per_rev;

    if keys.pressed(KeyCode::ArrowLeft) {
        sail_needs_update = true;
        sail_rot = rotate_speed * time.delta_secs();
    } else if keys.pressed(KeyCode::ArrowRight) {
        sail_needs_update = true;
        sail_rot = -rotate_speed * time.delta_secs();
    } 

    if sail_needs_update {
        for mut xf in q_sail.iter_mut() {
            xf.rotate_z(sail_rot);
        }
    }

    // WIND CHANGING //
    let mut wind_needs_update = false;
    let mut wind_delta = Vec2::ZERO;

 //   if keys.pressed(KeyCode::KeyA) {
 //       wind_needs_update = true;
 //       wind_delta.x = -constants.wind_change_speed * time.delta_secs();
 //   } else if keys.pressed(KeyCode::KeyD) {
 //       wind_needs_update = true;
 //       wind_delta.x = constants.wind_change_speed * time.delta_secs();
 //   } 
    if keys.pressed(KeyCode::KeyS) {
        wind_needs_update = true;
        wind_delta.y = -constants.wind_change_speed * time.delta_secs();
    } else if keys.pressed(KeyCode::KeyW) {
        wind_needs_update = true;
        wind_delta.y = constants.wind_change_speed * time.delta_secs();
    } 

    if wind_needs_update {
        wind.0 += wind_delta;
    }

    let mut turn_radius = f32::INFINITY;
    if keys.pressed(KeyCode::KeyA) {
        turn_radius = constants.boat_turn_radius;
    } else if keys.pressed(KeyCode::KeyD) {
        turn_radius = -constants.boat_turn_radius;
    } 
    for mut tr in &mut q_turn {
        tr.0 = turn_radius;
    }

}

#[derive(Reflect, Default, Resource)]
#[reflect(Resource)]
struct MousePos {
    last_left: Vec2,
    last_right: Vec2,
    current_left: Vec2,
    current_right: Vec2,
    left_pressed: bool,
    right_pressed: bool,
}


fn sys_mouse_track(q_windows: Query<&Window, With<PrimaryWindow>>,
                   buttons: Res<ButtonInput<MouseButton>>,
                   mut mouse_pos: ResMut<MousePos>) {
    let Some(pos) = q_windows.single().unwrap().cursor_position() else { return; };

    if buttons.just_pressed(MouseButton::Left) {
        mouse_pos.last_left = pos;
    }
    if buttons.just_pressed(MouseButton::Right) {
        mouse_pos.last_right = pos;
    }
    if buttons.pressed(MouseButton::Left) {
        mouse_pos.current_left = pos;
        mouse_pos.left_pressed = true;
    } else {
        mouse_pos.left_pressed = false;
    }
    if buttons.pressed(MouseButton::Right) {
        mouse_pos.current_right = pos;
        mouse_pos.right_pressed = true;
    } else {
        mouse_pos.right_pressed = false;
    }
}
