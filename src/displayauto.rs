use bevy::{color::palettes::css::*, prelude::*};
use bevy_prototype_lyon::prelude::*;
use serde::{Deserialize, Serialize};

const UNITS_SCALE: Vec3 = Vec3::new(
    1.0 / crate::UNITS_SCALE_FACTOR,
    1.0 / crate::UNITS_SCALE_FACTOR,
    1.0,
);
pub const ORIGIN_OFFSET: Vec2 = Vec2::new(4.02, -4.39);

const FILL: Srgba = WHITE;
const STROKE: Srgba = BLUE_VIOLET;

pub struct AutoPlugin;
impl Plugin for AutoPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DrawnWaypoints::default())
            .add_systems(Startup, init_waypoints.before(plot_waypoints))
            .add_systems(Startup, plot_waypoints);
    }
}

fn init_waypoints(mut commands: Commands) {
    type E = Box<dyn std::error::Error>;
    let mut wpts: Waypoints = serde_json::from_str(include_str!("../assets/field_waypoints.json"))
        .map_err(E::from)
        .unwrap();
    for node in wpts.wpts.iter_mut() {
        node.pos.y *= -1.0;
        std::mem::swap(&mut node.pos.x, &mut node.pos.y);
        node.pos += ORIGIN_OFFSET;
    }
    commands.insert_resource(wpts);
}

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct Waypoints {
    pub wpts: Vec<Waypoint>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Waypoint {
    pub name: String,
    pub pos: Vec2,
}

#[derive(Resource, Default)]
struct DrawnWaypoints {
    nodes: Vec<Entity>,
}

fn plot_waypoints(
    graph: Res<Waypoints>,
    mut drawn: ResMut<DrawnWaypoints>,
    mut commands: Commands,
) {
    for node in graph.wpts.iter() {
        drawn
            .nodes
            .push(draw_waypoint(node.pos, STROKE, FILL, &mut commands));
    }
}

fn draw_waypoint(
    node: Vec2,
    stroke_color: Srgba,
    fill_color: Srgba,
    commands: &mut Commands,
) -> Entity {
    let shape = shapes::Circle {
        radius: 25.0,
        center: node * crate::UNITS_SCALE_FACTOR,
    };
    commands
        .spawn((
            ShapeBundle {
                path: GeometryBuilder::build_as(&shape),
                ..Default::default()
            },
            Fill::color(fill_color),
            Stroke::new(stroke_color, 8.0),
        ))
        .insert(Transform::from_xyz(0.0, 0.0, 0.2).with_scale(UNITS_SCALE))
        .id()
}
