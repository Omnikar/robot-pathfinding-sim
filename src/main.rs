mod graph;
mod physics;
mod robot;

use bevy::prelude::*;

const BG_SCALE_FACTOR: f32 = 0.5;
const UNITS_SCALE_FACTOR: f32 = 237.18072;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Robot Pathfinding Sim".to_owned(),
                resizable: false,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(bevy_prototype_lyon::plugin::ShapePlugin)
        .add_plugins((
            graph::FieldGraphPlugin,
            physics::PhysicsPlugin,
            robot::RobotPlugin,
        ))
        .add_systems(Startup, (add_camera, set_background))
        .insert_resource(MouseWorldPos(Vec2::ZERO))
        .add_systems(Update, (set_window_size, mouse_hover))
        .run();
}

fn add_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_scale(Vec3::new(
            (BG_SCALE_FACTOR * UNITS_SCALE_FACTOR).recip(),
            (BG_SCALE_FACTOR * UNITS_SCALE_FACTOR).recip(),
            1.0,
        )),
        ..Default::default()
    });
}

#[derive(Resource)]
struct BackgroundHandle(Handle<Image>);

fn set_background(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("red-half-field.png");
    commands.insert_resource(BackgroundHandle(texture.clone()));
    commands.spawn(SpriteBundle {
        texture,
        transform: Transform::from_scale(Vec3::new(
            UNITS_SCALE_FACTOR.recip(),
            UNITS_SCALE_FACTOR.recip(),
            1.0,
        )),
        ..Default::default()
    });
}

fn set_window_size(
    background_handle: Res<BackgroundHandle>,
    mut windows: Query<&mut Window>,
    images: Res<Assets<Image>>,
) {
    if let Some(background) = images.get(&background_handle.0) {
        let size = background.size_f32() * BG_SCALE_FACTOR;
        let mut window = windows.single_mut();
        window.resolution.set(size.x, size.y);
    }
}

#[derive(Resource)]
struct MouseWorldPos(Vec2);

fn mouse_hover(
    mut mouse_world_pos: ResMut<MouseWorldPos>,
    window_q: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut reader: EventReader<CursorMoved>,
) {
    // Take the last mouse move event to get the most up-to-date position.
    let Some(event) = reader.read().last() else {
        return;
    };
    let screen_pos = event.position;

    let (camera, camera_transform) = camera_q.single();

    let window = window_q.single();
    let window_size = Vec2::new(window.width(), window.height());
    let ndc = screen_pos / window_size * 2.0 - Vec2::ONE;
    let ndc_to_world = camera_transform.compute_matrix() * camera.clip_from_view().inverse();
    let mut world_pos = ndc_to_world.project_point3(ndc.extend(-1.0)).truncate();
    world_pos.y = -world_pos.y;

    mouse_world_pos.0 = world_pos;

    let world_pos_rounded = (world_pos * 1e2).round() / 1e2;
    use std::io::Write;
    print!("\r{},{}\x1b[J", world_pos_rounded.x, world_pos_rounded.y);
    std::io::stdout().flush().unwrap();
}
