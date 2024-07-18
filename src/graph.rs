use std::collections::HashSet as Set;
use std::io::Write;

use bevy::{color::palettes::css::*, prelude::*};
use bevy_prototype_lyon::prelude::*;

use serde::{Deserialize, Serialize};

use crate::{Mode, MouseWorldPos, SavePath};

const UNITS_SCALE: Vec3 = Vec3::new(
    1.0 / crate::UNITS_SCALE_FACTOR,
    1.0 / crate::UNITS_SCALE_FACTOR,
    1.0,
);

const FILL: Srgba = WHITE;
const STROKE: Srgba = GREEN;
const HIGHLIGHT: Srgba = SKY_BLUE;
const NEG_HIGHLIGHT: Srgba = RED;

pub struct FieldGraphPlugin;
impl Plugin for FieldGraphPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DrawnGraph::default())
            .insert_resource(EditState::Normal)
            .insert_resource(Hovered::default())
            .add_systems(Startup, init_field_graph.before(draw_field_graph))
            .add_systems(Startup, draw_field_graph)
            .add_systems(
                Update,
                (
                    update_mouse_state,
                    mouse_interaction.after(update_mouse_state),
                )
                    .run_if(in_state(Mode::EditGraph)),
            )
            .add_systems(Update, save_field_graph)
            .add_systems(OnExit(Mode::EditGraph), on_exit_edit_mode);
    }
}

fn init_field_graph(save_path: Res<SavePath>, mut commands: Commands) {
    type E = Box<dyn std::error::Error>;
    let graph = match std::fs::File::open(&save_path.0)
        .map_err(E::from)
        .and_then(|f| serde_json::from_reader(f).map_err(E::from))
    {
        Ok(graph) => graph,
        Err(_) => SpatialGraph {
            nodes: vec![
                Vec2::new(0.0, 0.60),
                Vec2::new(2.19, -0.26),
                Vec2::new(2.25, 1.58),
                Vec2::new(0.0, 2.12),
                Vec2::new(-1.99, 1.70),
                Vec2::new(-1.47, -0.45),
                Vec2::new(-0.45, -1.44),
                Vec2::new(1.66, -1.42),
            ],
            edges: vec![
                (0, 1),
                (0, 3),
                (0, 5),
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 1),
            ],
        },
    };
    commands.insert_resource(FieldGraph(graph));
}

#[derive(Resource)]
pub struct FieldGraph(pub SpatialGraph);

#[derive(Clone, Serialize, Deserialize)]
pub struct SpatialGraph {
    pub nodes: Vec<Vec2>,
    pub edges: Vec<(usize, usize)>,
}

impl SpatialGraph {
    /// Finds a given edge, if it exists. Disregards order of node indices in tuple.
    fn find_edge(&self, (start_i, end_i): (usize, usize)) -> Option<usize> {
        self.edges
            .iter()
            .enumerate()
            .find_map(|(i, &tup)| (tup == (start_i, end_i) || tup == (end_i, start_i)).then_some(i))
    }

    fn connected_edges(&self, node_i: usize) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.edges
            .iter()
            .copied()
            .enumerate()
            .filter(move |&(_, (a, b))| a == node_i || b == node_i)
            .map(|tup| tup.0)
    }
}

#[derive(Resource, Default)]
struct DrawnGraph {
    nodes: Vec<Entity>,
    edges: Vec<Entity>,
}

fn draw_field_graph(graph: Res<FieldGraph>, mut drawn: ResMut<DrawnGraph>, mut commands: Commands) {
    for &node in &graph.0.nodes {
        drawn
            .nodes
            .push(draw_node(node, STROKE, FILL, &mut commands));
    }
    for &edge in &graph.0.edges {
        let p1 = graph.0.nodes[edge.0];
        let p2 = graph.0.nodes[edge.1];
        drawn.edges.push(draw_edge(p1, p2, STROKE, &mut commands));
    }
}

fn draw_node(
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

fn draw_edge(p1: Vec2, p2: Vec2, color: Srgba, commands: &mut Commands) -> Entity {
    let zmod = match color {
        NEG_HIGHLIGHT => 0.12,
        HIGHLIGHT => 0.11,
        _ => 0.1,
    };
    let shape = shapes::Line(p1, p2);
    commands
        .spawn((
            ShapeBundle {
                path: GeometryBuilder::build_as(&shape),
                ..Default::default()
            },
            Stroke::new(color, 10.0 / crate::UNITS_SCALE_FACTOR),
        ))
        .insert(Transform::from_xyz(0.0, 0.0, zmod))
        .id()
}

fn replace_node(
    i: usize,
    stroke: Srgba,
    fill: Srgba,
    graph: &FieldGraph,
    drawn: &mut DrawnGraph,
    commands: &mut Commands,
) {
    if let Some(&id) = drawn.nodes.get(i) {
        commands.entity(id).despawn();
        drawn.nodes[i] = draw_node(graph.0.nodes[i], stroke, fill, commands);
    }
}

fn replace_edge(
    i: usize,
    color: Srgba,
    graph: &FieldGraph,
    drawn: &mut DrawnGraph,
    commands: &mut Commands,
) {
    if let Some(&id) = drawn.edges.get(i) {
        commands.entity(id).despawn();
        let edge = graph.0.edges[i];
        drawn.edges[i] = draw_edge(
            graph.0.nodes[edge.0],
            graph.0.nodes[edge.1],
            color,
            commands,
        );
    }
}

fn split_edges<T: std::ops::Deref<Target = usize>>(
    new_i: usize,
    edges: impl IntoIterator<Item = T>,
    graph: &mut FieldGraph,
    drawn: &mut DrawnGraph,
    commands: &mut Commands,
) {
    let mut edges_to_replace: Vec<_> = edges.into_iter().map(|x| *x).collect();
    edges_to_replace.sort_unstable();
    for edge_i in edges_to_replace.into_iter().rev() {
        let (start_i, end_i) = graph.0.edges.remove(edge_i);
        let (start, end) = (graph.0.nodes[start_i], graph.0.nodes[end_i]);

        graph.0.edges.push((start_i, new_i));
        graph.0.edges.push((new_i, end_i));

        commands.entity(drawn.edges.remove(edge_i)).despawn();
        drawn
            .edges
            .push(draw_edge(start, graph.0.nodes[new_i], STROKE, commands));
        drawn
            .edges
            .push(draw_edge(graph.0.nodes[new_i], end, STROKE, commands));
    }
}

#[derive(Resource, Clone, Copy)]
enum EditState {
    Normal,
    MakingEdge(usize, Option<Entity>),
    DraggingNode(usize, Vec2),
}

#[derive(Resource, Clone, Default)]
struct Hovered {
    // (index of hovered, index of highlighted)
    node: (Option<usize>, Option<usize>),
    edges: (Set<usize>, Set<usize>),
}

struct MouseDragDetector {
    timer: Timer,
    click_pos: Option<Vec2>,
}

impl Default for MouseDragDetector {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
            click_pos: None,
        }
    }
}

impl MouseDragDetector {
    fn dragging(&self, mouse_pos: Vec2) -> bool {
        self.click_pos
            .is_some_and(|p| (p - mouse_pos).length() > 0.1)
            || self.timer.finished()
    }
}

// Updates stored hovered node and dragging state
#[allow(clippy::too_many_arguments)]
fn update_mouse_state(
    mouse_pos: Res<MouseWorldPos>,
    mouse_click: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    graph: Res<FieldGraph>,
    mut edit_state: ResMut<EditState>,
    mut drawn: ResMut<DrawnGraph>,
    mut hovered: ResMut<Hovered>,
    mut drag_detector: Local<MouseDragDetector>,
    mut commands: Commands,
) {
    drag_detector.timer.tick(time.delta());

    if mouse_click.just_pressed(MouseButton::Left) {
        drag_detector.click_pos = Some(mouse_pos.0);
        drag_detector.timer.unpause();
    } else if !mouse_click.pressed(MouseButton::Left) {
        drag_detector.click_pos = None;
        drag_detector.timer.pause();
        drag_detector.timer.reset();
    }

    let find_hovered_node = |pos: Vec2| {
        graph
            .0
            .nodes
            .iter()
            .copied()
            .map(|n| n - pos)
            .map(Vec2::length)
            .enumerate()
            .filter(|&(_, dist)| dist < 0.13)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|tup| tup.0)
    };

    let dragging = drag_detector.dragging(mouse_pos.0);
    match (dragging, matches!(*edit_state, EditState::DraggingNode(..))) {
        (true, false) => {
            let click_pos = drag_detector.click_pos.unwrap_or(mouse_pos.0);
            if let Some(i) = find_hovered_node(click_pos) {
                let relative_pos = graph.0.nodes[i] - click_pos;
                if let EditState::MakingEdge(_, Some(id)) = *edit_state {
                    commands.entity(id).despawn();
                }
                *edit_state = EditState::DraggingNode(i, relative_pos);
            }
        }
        (false, true) => {
            *edit_state = EditState::Normal;
        }
        _ => {}
    }

    hovered.node.0 = find_hovered_node(mouse_pos.0);
    if !dragging {
        // Handle hovered node coloring
        if hovered.node.0 != hovered.node.1 {
            if let Some(i) = hovered.node.1 {
                replace_node(i, STROKE, FILL, &graph, &mut drawn, &mut commands);
            }
            if let Some(i) = hovered.node.0 {
                replace_node(i, HIGHLIGHT, FILL, &graph, &mut drawn, &mut commands);
            }
        }
        hovered.node.1 = hovered.node.0;
    }

    hovered.edges.0 =
        if hovered.node.0.is_none() && !matches!(*edit_state, EditState::DraggingNode(..)) {
            graph
                .0
                .edges
                .iter()
                .enumerate()
                .filter_map(|(i, &(start_i, end_i))| {
                    let (start, end) = (graph.0.nodes[start_i], graph.0.nodes[end_i]);
                    let (vec1, vec2) = (start - mouse_pos.0, end - mouse_pos.0);
                    (vec1.perp_dot(vec2).abs() < 0.1 && vec1.dot(vec2) < 0.0).then_some(i)
                })
                .collect()
        } else {
            Set::new()
        };
    // Highlight unhighlighted but hovered edges
    for &edge_i in hovered.edges.0.difference(&hovered.edges.1) {
        replace_edge(edge_i, HIGHLIGHT, &graph, &mut drawn, &mut commands);
    }
    // Unhighlight highlighted but not hovered edges
    for &edge_i in hovered.edges.1.difference(&hovered.edges.0) {
        replace_edge(edge_i, STROKE, &graph, &mut drawn, &mut commands);
    }
    // This `let` sequence is to appease the borrow checker.
    // Otherwise it attempts to take two separate borrows to `hovered.edges`,
    // rather than taking borrows to two separate fields from the same mutable
    // borrow of `hovered.edges`.
    let edges = &mut hovered.edges;
    let (hovered_edges, highlighted_edges) = (&edges.0, &mut edges.1);
    highlighted_edges.clone_from(hovered_edges);
}

#[allow(clippy::too_many_arguments)]
fn mouse_interaction(
    mouse_pos: Res<MouseWorldPos>,
    mouse_click: Res<ButtonInput<MouseButton>>,
    hovered: Res<Hovered>,
    mut edit_state: ResMut<EditState>,
    mut graph: ResMut<FieldGraph>,
    mut drawn: ResMut<DrawnGraph>,
    mut commands: Commands,
) {
    match (*edit_state, hovered.node.0) {
        // Clicked on a node - start drawing an edge from it
        (EditState::Normal, Some(i)) if mouse_click.just_pressed(MouseButton::Left) => {
            *edit_state = EditState::MakingEdge(i, None);
        }
        // Right clicked a node - delete it and all connecting edges
        (EditState::Normal, Some(i)) if mouse_click.just_pressed(MouseButton::Right) => {
            // Reverse index list so that sequential deletion doesn't shift the indices being affected
            let edges_to_delete = graph.0.connected_edges(i).rev().collect::<Vec<_>>();
            let del_edge = |i| {
                graph.0.edges.remove(i);
                commands.entity(drawn.edges.remove(i)).despawn();
            };
            edges_to_delete.into_iter().for_each(del_edge);
            graph
                .0
                .edges
                .iter_mut()
                .flat_map(|(a, b)| [a, b])
                .filter(|v| **v > i)
                .for_each(|v| *v -= 1);

            graph.0.nodes.remove(i);
            commands.entity(drawn.nodes.remove(i)).despawn();
        }
        // Clicked empty space - create a new node and start drawing an edge from it
        (EditState::Normal, None) if mouse_click.just_pressed(MouseButton::Left) => {
            let new_i = graph.0.nodes.len();
            graph.0.nodes.push(mouse_pos.0);
            drawn
                .nodes
                .push(draw_node(mouse_pos.0, STROKE, FILL, &mut commands));

            split_edges(
                new_i,
                &hovered.edges.0,
                &mut graph,
                &mut drawn,
                &mut commands,
            );

            *edit_state = EditState::MakingEdge(new_i, None);
        }
        // Right clicked empty space - delete all hovered edges
        (EditState::Normal, None) if mouse_click.just_pressed(MouseButton::Right) => {
            let mut edges_to_delete: Vec<_> = hovered.edges.0.iter().copied().collect();
            edges_to_delete.sort_unstable();
            for i in edges_to_delete.into_iter().rev() {
                graph.0.edges.remove(i);
                commands.entity(drawn.edges.remove(i)).despawn();
            }
        }
        // Clicked on a node while drawing an edge - create the edge if it doesn't exist already, remove the edge if it does
        (EditState::MakingEdge(start_i, id_o), Some(end_i))
            if mouse_click.just_pressed(MouseButton::Left) =>
        {
            if let Some(id) = id_o {
                commands.entity(id).despawn();
            }
            if let Some(existing_edge) = graph.0.find_edge((start_i, end_i)) {
                graph.0.edges.remove(existing_edge);
                commands.entity(drawn.edges.remove(existing_edge)).despawn();
            } else {
                graph.0.edges.push((start_i, end_i));
                drawn.edges.push(draw_edge(
                    graph.0.nodes[start_i],
                    graph.0.nodes[end_i],
                    STROKE,
                    &mut commands,
                ));
            }
            *edit_state = EditState::Normal;
        }
        // Clicked empty space while drawing an edge - create a new node and create the edge connected to it
        (EditState::MakingEdge(start_i, id_o), None)
            if mouse_click.just_pressed(MouseButton::Left) =>
        {
            if let Some(id) = id_o {
                commands.entity(id).despawn();
            }
            let end_i = graph.0.nodes.len();
            graph.0.nodes.push(mouse_pos.0);
            graph.0.edges.push((start_i, end_i));
            drawn
                .nodes
                .push(draw_node(mouse_pos.0, STROKE, FILL, &mut commands));
            drawn.edges.push(draw_edge(
                graph.0.nodes[start_i],
                mouse_pos.0,
                STROKE,
                &mut commands,
            ));

            split_edges(
                end_i,
                &hovered.edges.0,
                &mut graph,
                &mut drawn,
                &mut commands,
            );

            *edit_state = EditState::Normal;
        }
        // Right clicked while drawing an edge - cancel the edge drawing
        (EditState::MakingEdge(_, id_o), _) if mouse_click.just_pressed(MouseButton::Right) => {
            if let Some(id) = id_o {
                commands.entity(id).despawn();
            }
            *edit_state = EditState::Normal;
        }
        // Idle while making edge - highlight accordingly and snap to hovered node, if any
        (EditState::MakingEdge(start_i, id_o), hovered_node) => {
            if let Some(id) = id_o {
                commands.entity(id).despawn();
            }
            let is_edge_deletion =
                hovered_node.is_some_and(|end_i| graph.0.find_edge((start_i, end_i)).is_some());
            let id = draw_edge(
                graph.0.nodes[start_i],
                match hovered_node {
                    Some(i) => graph.0.nodes[i],
                    None => mouse_pos.0,
                },
                if is_edge_deletion {
                    NEG_HIGHLIGHT
                } else {
                    HIGHLIGHT
                },
                &mut commands,
            );
            *edit_state = EditState::MakingEdge(start_i, Some(id));
        }
        // Dragging a node
        (EditState::DraggingNode(i, relative_pos), _) => {
            let new_pos = mouse_pos.0 + relative_pos;
            graph.0.nodes[i] = new_pos;

            replace_node(i, HIGHLIGHT, FILL, &graph, &mut drawn, &mut commands);

            for edge_i in graph.0.connected_edges(i) {
                commands.entity(drawn.edges[edge_i]).despawn();
                let edge = graph.0.edges[edge_i];
                drawn.edges[edge_i] = draw_edge(
                    graph.0.nodes[edge.0],
                    graph.0.nodes[edge.1],
                    STROKE,
                    &mut commands,
                );
            }
        }
        _ => {}
    }
}

fn on_exit_edit_mode(
    mut edit_state: ResMut<EditState>,
    hovered: Res<Hovered>,
    graph: Res<FieldGraph>,
    mut drawn: ResMut<DrawnGraph>,
    mut commands: Commands,
) {
    if let EditState::MakingEdge(_, Some(id)) = *edit_state {
        commands.entity(id).despawn();
    }
    if let Some(i) = hovered.node.1 {
        replace_node(i, STROKE, FILL, &graph, &mut drawn, &mut commands);
    }
    for &i in &hovered.edges.1 {
        replace_edge(i, STROKE, &graph, &mut drawn, &mut commands);
    }
    *edit_state = EditState::Normal;
}

fn save_field_graph(
    graph: Res<FieldGraph>,
    save_path: Res<SavePath>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::KeyS) {
        return;
    }

    // Weird error juggling shenanigans (rust devs stabilize try blocks pls)
    type E = Box<dyn std::error::Error>;
    if let Err(e) = serde_json::to_string_pretty(&graph.0)
        .map_err(E::from)
        .and_then(|serialized| {
            std::fs::File::create(&save_path.0)
                .and_then(|mut f| write!(f, "{serialized}"))
                .map_err(E::from)
        })
    {
        eprintln!("{e}");
    } else {
        eprintln!("Saved to {}", save_path.0);
    }
}
