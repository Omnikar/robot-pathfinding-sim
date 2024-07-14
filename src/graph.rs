use bevy::{color::palettes::css::*, prelude::*};
use bevy_prototype_lyon::prelude::*;

const UNITS_SCALE: Vec3 = Vec3::new(
    1.0 / crate::UNITS_SCALE_FACTOR,
    1.0 / crate::UNITS_SCALE_FACTOR,
    1.0,
);

const FILL: Srgba = WHITE;
const STROKE: Srgba = GREEN;

pub struct FieldGraphPlugin;
impl Plugin for FieldGraphPlugin {
    fn build(&self, app: &mut App) {
        let graph = SpatialGraph {
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
        };
        app.insert_resource(FieldGraph(graph))
            .add_systems(Startup, draw_field_graph);
    }
}

#[derive(Resource)]
pub struct FieldGraph(pub SpatialGraph);

#[derive(Clone)]
pub struct SpatialGraph {
    pub nodes: Vec<Vec2>,
    pub edges: Vec<(usize, usize)>,
}

fn draw_field_graph(graph: Res<FieldGraph>, mut commands: Commands) {
    for &node in &graph.0.nodes {
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
                Fill::color(FILL),
                Stroke::new(STROKE, 8.0),
            ))
            .insert(Transform::from_xyz(0.0, 0.0, 0.2).with_scale(UNITS_SCALE));
    }
    for &edge in &graph.0.edges {
        let p1 = graph.0.nodes[edge.0];
        let p2 = graph.0.nodes[edge.1];
        let shape = shapes::Line(p1, p2);
        commands
            .spawn((
                ShapeBundle {
                    path: GeometryBuilder::build_as(&shape),
                    ..Default::default()
                },
                Stroke::new(STROKE, 10.0 / crate::UNITS_SCALE_FACTOR),
            ))
            .insert(Transform::from_xyz(0.0, 0.0, 0.1));
    }
}
