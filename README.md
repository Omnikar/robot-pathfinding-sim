# FRC robot pathfinding simulation
Run with `cargo` (saved json path is optional and will default to `graph.json`):
```bash
cargo run -- [saved json path]
```
Left click to set the robot target.
Right click to teleport the robot.
World coordinates of the mouse are displayed in the command line.
The robot pathfinds along the superimposed graph to get from its position to its destination.

## Edit mode
Toggle edit mode by pressing `e`.
In edit mode:
- Click two nodes to draw an edge between them.
  - You can click empty space instead of an existing node to create a new node.
    - You can click on an edge instead of empty space to create a new node on that edge.
- While drawing an edge, right click to cancel.
- Right click a node to delete it.
- Right click an edge to delete it.
- Click both nodes of an existing edge to delete that edge.
- Click and drag a node to move it.

Save the graph as a json file by pressing `s`.

![Pathfinding example](/example.png)
