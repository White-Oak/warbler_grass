use bevy::{prelude::*, window::PresentMode};
use bevy_obj::*;
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
use rand::{thread_rng, Rng};
use warbler_grass::grass_spawner::GrassSpawner;
use warbler_grass::{warblers_plugin::WarblersPlugin, GrassConfiguration, WarblersBundle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::window::WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugin(WarblersPlugin)
        .add_plugin(ObjPlugin)
        .add_plugin(ScreenDiagnosticsPlugin::default())
        .add_plugin(ScreenFrameDiagnosticsPlugin)
        .add_plugin(SimpleCamera)
        .add_startup_system(setup_grass)
        .run();
}

pub struct SimpleCamera;
impl Plugin for SimpleCamera {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_camera)
            .add_system(camera_movement);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-5.0, 4., -5.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });
}

fn camera_movement(input: Res<Input<KeyCode>>, mut query: Query<&mut Transform, With<Camera>>) {
    for mut transform in &mut query {
        let move_speed = 0.2 / 4.;
        let rotate_speed = 0.02 / 4.;
        let mut forward = transform.forward();
        let up = transform.up();
        forward.y = 0.;
        let right = transform.right();

        if input.pressed(KeyCode::W) {
            transform.translation += forward * move_speed;
        }
        if input.pressed(KeyCode::S) {
            transform.translation -= forward * move_speed;
        }
        if input.pressed(KeyCode::Q) {
            transform.rotate_y(-rotate_speed);
        }
        if input.pressed(KeyCode::E) {
            transform.rotate_y(rotate_speed);
        }
        if input.pressed(KeyCode::A) {
            transform.translation -= right * move_speed;
        }
        if input.pressed(KeyCode::D) {
            transform.translation += right * move_speed;
        }
        if input.pressed(KeyCode::Z) {
            transform.translation -= up * move_speed;
        }
        if input.pressed(KeyCode::X) {
            transform.translation += up * move_speed;
        }
    }
}

const SIDE: usize = 300;
const FULL: usize = SIDE * SIDE;

#[allow(dead_code)]
pub fn get_grass_grid() -> GrassSpawner {
    // let positions = vec![Vec3::splat(0.)];
    let mut rng = thread_rng();
    let positions = (0..FULL)
        .map(|i| {
            Vec3::new(
                (i / SIDE) as f32 + (rng.gen::<f32>() / 1.5),
                0.,
                (i % SIDE) as f32,
            ) / 2.
                + rng.gen::<f32>() / 2.
        })
        .collect();
    let heights = (0..FULL).map(|_| 0.5 + (rng.gen::<f32>() / 2.)).collect();
    GrassSpawner::new()
        .with_positions(positions)
        .with_heights(heights)
}

fn setup_grass(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut grass_config: ResMut<GrassConfiguration>
) {
    let main_color = Color::hex("497c46").unwrap().as_linear_rgba_f32();
    let main_color = Color::from(main_color);
    let bottom_color = Color::hex("45763e").unwrap().as_linear_rgba_f32();
    let bottom_color = Color::from(bottom_color);
    grass_config.main_color = main_color;
    grass_config.bottom_color = bottom_color;
    grass_config.wind = Vec2::new(0., 2.);

    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(1000.0).into()),
        material: materials.add(Color::rgb(0.6, 0.7, 0.8).into()),
        transform: Transform::from_xyz(0., 0.2, 0.),
        ..default()
    });

    let grass_mesh = asset_server.load("grass.obj");
    commands.spawn((WarblersBundle {
        grass_spawner: get_grass_grid(),
        grass_mesh,
        ..default()
    },));
}
