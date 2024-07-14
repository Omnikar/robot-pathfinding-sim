use std::f32::consts::PI;

use bevy::{color::palettes::css::*, prelude::*};
use bevy_prototype_lyon::prelude::*;

use pathfinding::directed::astar::astar;

use crate::graph::{FieldGraph, SpatialGraph};
use crate::physics::{AngularVelocity, Velocity};
use crate::UNITS_SCALE_FACTOR;

const ROBOT_COLOR: Srgba = RED;
const ROBOT_BORDER_COLOR: Srgba = DARK_RED;

pub struct RobotPlugin;
impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_robot)
            .add_event::<RecomputeRobotPath>()
            .add_systems(
                PostStartup,
                |mut writer: EventWriter<RecomputeRobotPath>| {
                    writer.send_default();
                },
            )
            .add_systems(
                Update,
                (follow_path, click_events, recompute_robot_path, face_target),
            );
    }
}

#[derive(Component)]
pub struct Robot;

#[derive(Component)]
pub struct PathFollower {
    target_path: Vec<Vec2>,
    p: f32,
    max_speed: f32,
}

#[derive(Component)]
pub struct TargetFacer {
    p: f32,
    max_speed: f32,
}

#[derive(Component)]
pub struct TargetPosition(Vec2);

fn spawn_robot(mut commands: Commands) {
    let shapes = {
        let rect_shape = shapes::Rectangle {
            extents: Vec2::new(0.62, 0.62),
            ..Default::default()
        };
        let rect = commands
            .spawn((
                ShapeBundle {
                    path: GeometryBuilder::build_as(&rect_shape),
                    ..Default::default()
                },
                Fill::color(ROBOT_COLOR),
                Stroke::new(ROBOT_BORDER_COLOR, 10.0 / UNITS_SCALE_FACTOR),
            ))
            .insert(Transform::from_xyz(0.0, 0.0, 1.0))
            .id();
        let line_shape = shapes::Line(Vec2::ZERO, Vec2::new(0.2, 0.0));
        let line = commands
            .spawn((
                ShapeBundle {
                    path: GeometryBuilder::build_as(&line_shape),
                    ..Default::default()
                },
                Stroke::new(BLACK, 12.0 / UNITS_SCALE_FACTOR),
            ))
            .insert(Transform::from_xyz(0.0, 0.0, 1.1))
            .id();
        [rect, line]
    };

    let follower = PathFollower {
        target_path: Vec::new(),
        // Movement values
        p: 5.0,
        // m/s
        max_speed: 4.0,
    };
    let facer = TargetFacer {
        // Rotation values
        p: 5.0,
        // rad/s
        max_speed: 4.0,
    };
    let init_pos = Vec2::new(1.05, -1.9);
    let init_rot = PI / 2.0;
    commands
        .spawn((Robot, TargetPosition(init_pos), follower, facer))
        .push_children(&shapes)
        .insert(SpatialBundle {
            transform: Transform::from_xyz(init_pos.x, init_pos.y, 0.0)
                .with_rotation(Quat::from_rotation_z(init_rot)),
            ..Default::default()
        })
        .insert(Velocity(Vec2::new(0.0, 0.0)))
        .insert(AngularVelocity(0.0));
}

fn follow_path(mut q: Query<(&mut PathFollower, &mut Velocity, &Transform)>) {
    let (mut follower, mut vel, transform) = q.single_mut();
    let pos = transform.translation.truncate();

    let passthrough = |i| if i == 0 { 0.1 } else { 0.5 };

    let mut path_iter = follower.target_path.iter().copied().rev().enumerate().rev();
    let next_wp = path_iter.find(|&(i, wp)| (pos - wp).length() > passthrough(i));
    follower.target_path = path_iter.map(|t| t.1).collect();

    let Some((_, next_wp)) = next_wp else {
        vel.0 = Vec2::ZERO;
        return;
    };
    follower.target_path.insert(0, next_wp);

    let mut new_vel = follower.p * (next_wp - pos);
    if new_vel.length() > follower.max_speed {
        new_vel = follower.max_speed * new_vel.normalize();
    }
    vel.0 = new_vel;
}

fn face_target(
    mut q: Query<(
        &mut AngularVelocity,
        &Transform,
        &TargetFacer,
        &TargetPosition,
    )>,
) {
    let (mut avel, transform, facer, target) = q.single_mut();

    fn norm_angle(a: f32) -> f32 {
        (a + PI).rem_euclid(2.0 * PI) - PI
    }

    let pos_diff = target.0 - transform.translation.truncate();
    if pos_diff.length() < 0.1 {
        avel.0 = 0.0;
        return;
    }
    let target_angle = pos_diff.to_angle();

    let (axis, axis_angle) = transform.rotation.to_axis_angle();
    let cur_angle = axis.dot(Vec3::Z) * axis_angle;

    let diff = norm_angle(target_angle - cur_angle);
    let mut new_avel = facer.p * diff;
    if new_avel.abs() > facer.max_speed {
        new_avel = facer.max_speed * new_avel.signum();
    }
    avel.0 = new_avel;
}

#[derive(Event, Default)]
struct RecomputeRobotPath;

fn recompute_robot_path(
    mut q: Query<(&mut PathFollower, &TargetPosition, &Transform), With<Robot>>,
    graph: Res<FieldGraph>,
    mut reader: EventReader<RecomputeRobotPath>,
) {
    if reader.is_empty() {
        return;
    }

    let (mut follower, target, transform) = q.single_mut();
    follower.target_path =
        compute_path(transform.translation.truncate(), target.0, &graph.0).expect("no path found");

    reader.clear();
}

fn compute_path(start: Vec2, end: Vec2, graph: &SpatialGraph) -> Option<Vec<Vec2>> {
    let mut graph = graph.clone();
    let mut insert_node = |new_node: Vec2| {
        let closest = graph
            .nodes
            .iter()
            .map(|&node| (node - new_node).length())
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;
        let new_idx = graph.nodes.len();
        graph.nodes.push(new_node);
        graph.edges.push((closest, new_idx));
    };
    insert_node(start);
    insert_node(end);

    let end_idx = graph.nodes.len() - 1;

    let dist_cost = |a: Vec2, b: Vec2| ((a - b).length() * 1e5).round() as u32;

    let successors = |&i: &usize| {
        let mut succ = Vec::new();
        for &(a, b) in &graph.edges {
            if a == i {
                succ.push(b);
            }
            if b == i {
                succ.push(a);
            }
        }
        succ.into_iter()
            .map(|n| (n, dist_cost(graph.nodes[i], graph.nodes[n])))
            .collect::<Vec<_>>()
    };

    let heuristic = |&i: &usize| dist_cost(graph.nodes[i], end);

    let path = astar(&(end_idx - 1), successors, heuristic, |&i| i == end_idx);

    path.map(|t| t.0.into_iter().map(|i| graph.nodes[i]).collect())
}

fn click_events(
    mut robot_q: Query<(&mut Transform, &mut TargetPosition), With<Robot>>,
    mouse_pos: Res<crate::MouseWorldPos>,
    mouse_click: Res<ButtonInput<MouseButton>>,
    mut writer: EventWriter<RecomputeRobotPath>,
) {
    let (mut transform, mut target) = robot_q.single_mut();

    let mut updated = true;
    if mouse_click.just_pressed(MouseButton::Left) {
        target.0 = mouse_pos.0;
    } else if mouse_click.just_pressed(MouseButton::Right) {
        transform.translation.x = mouse_pos.0.x;
        transform.translation.y = mouse_pos.0.y;
    } else {
        updated = false;
    }

    if updated {
        writer.send_default();
    }
}
