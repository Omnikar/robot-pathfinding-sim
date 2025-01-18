use crate::graph::{FieldGraph, SpatialGraph};
use crate::AutoPath;
use bevy::{color::palettes::css::*, prelude::*};
use bevy_prototype_lyon::prelude::*;
use serde::{Deserialize, Serialize};

const FILL: Srgba = WHITE;
const STROKE: Srgba = BLUE_VIOLET;

pub struct AutoPlugin;
impl Plugin for AutoPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DrawnWaypoints::default())
            .insert_resource(DrawnAuto::default())
            .add_systems(Startup, init_waypoints.before(plot_waypoints))
            .add_systems(Startup, plot_waypoints)
            .add_systems(Startup, init_auto.before(draw_auto))
            .add_systems(Startup, draw_auto);
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
        node.pos += crate::graph::ORIGIN_OFFSET;
    }
    commands.insert_resource(wpts);
}

fn init_auto(auto_path: Res<AutoPath>, mut commands: Commands) {
    type E = Box<dyn std::error::Error>;
    let auto = std::fs::File::open(&auto_path.0)
        .map_err(E::from)
        .and_then(|f| serde_json::from_reader::<_, Auto>(f).map_err(E::from))
        .unwrap();
    commands.insert_resource(auto);
}

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct Auto {
    pub wpts: Vec<String>,
}

#[derive(Resource, Default)]
pub struct DrawnAuto {
    pub edges: Vec<Entity>,
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
        drawn.nodes.push(crate::graph::draw_node(
            node.pos,
            STROKE,
            FILL,
            &mut commands,
        ));
    }
}

fn draw_auto(
    auto: Res<Auto>,
    wpts: Res<Waypoints>,
    graph: Res<FieldGraph>,
    mut drawn: ResMut<DrawnAuto>,
    mut commands: Commands,
) {
    let mut prev = match_waypoint("robostart", &wpts);
    for current in auto.wpts.iter().map(|x| match_waypoint(&x, &wpts)) {
        let Some(path) = crate::robot::compute_path(prev.pos, current.pos, &graph.sg) else {
            return;
        };
        let mut prev_1 = prev.pos;
        for current_1 in path.iter() {
            drawn.edges.push(crate::graph::draw_edge(
                prev_1,
                *current_1,
                STROKE,
                &mut commands,
            ));
            prev_1 = *current_1;
        }
        prev = current;
    }
}

fn match_waypoint<'a>(auto: &str, wpts: &'a Waypoints) -> &'a Waypoint {
    wpts.wpts
        .iter()
        .find(|x| x.name == auto)
        .unwrap_or_else(|| panic!("No waypoint found {}", auto))
}
