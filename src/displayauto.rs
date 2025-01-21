use crate::graph::FieldGraph;
use crate::AutoPath;
use bevy::{color::palettes::css::*, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Resource, Serialize, Deserialize)]
pub struct Waypoints(HashMap<String, Vec2>);
impl std::ops::Deref for Waypoints {
    type Target = HashMap<String, Vec2>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Waypoints {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn init_waypoints(mut commands: Commands) {
    type E = Box<dyn std::error::Error>;
    let mut wpts: Waypoints = serde_json::from_str(include_str!("../assets/field_waypoints.json"))
        .map_err(E::from)
        .unwrap();
    for node in wpts.values_mut() {
        node.y *= -1.0;
        std::mem::swap(&mut node.x, &mut node.y);
        *node += crate::graph::ORIGIN_OFFSET;
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

#[derive(Resource, Default)]
struct DrawnWaypoints {
    nodes: Vec<Entity>,
}

fn plot_waypoints(
    graph: Res<Waypoints>,
    mut drawn: ResMut<DrawnWaypoints>,
    mut commands: Commands,
) {
    for node in graph.iter() {
        drawn.nodes.push(crate::graph::draw_node(
            *node.1,
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
    let path = gen_auto_path(&auto, &wpts, &graph);
    for edge in path.windows(2) {
        drawn.edges.push(crate::graph::draw_edge(
            edge[0],
            edge[1],
            STROKE,
            &mut commands,
        ));
    }
}

pub fn gen_auto_path(auto: &Auto, wpts: &Waypoints, graph: &FieldGraph) -> Vec<Vec2> {
    let mut path: Vec<Vec2> = Vec::new();
    path.push(*wpts.get("robostart").expect("Where you starting???"));
    for (i, current) in auto
        .wpts
        .iter()
        .map(|x| wpts.get(x).expect("nada"))
        .enumerate()
    {
        let mut subpath =
            crate::robot::compute_path(*path.last().expect("nothin in this"), *current, &graph.sg)
                .expect("No path found, silly");
        path.append(&mut subpath);
    }
    return path;
}
