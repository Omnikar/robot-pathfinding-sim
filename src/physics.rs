use bevy::prelude::*;

pub struct PhysicsPlugin;
impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_velocity, apply_angular_velocity));
    }
}

#[derive(Component, Clone, Copy)]
pub struct Velocity(pub Vec2);

fn apply_velocity(mut q: Query<(&Velocity, &mut Transform)>, time: Res<Time>) {
    let delta_t = time.delta_seconds();
    for (&vel, mut transform) in q.iter_mut() {
        let delta = delta_t * vel.0;
        transform.translation += Vec3::new(delta.x, delta.y, 0.0);
    }
}

#[derive(Component, Clone, Copy)]
pub struct AngularVelocity(pub f32);

fn apply_angular_velocity(mut q: Query<(&AngularVelocity, &mut Transform)>, time: Res<Time>) {
    let delta_t = time.delta_seconds();
    for (&vel, mut transform) in q.iter_mut() {
        let delta = delta_t * vel.0;
        transform.rotate_z(delta);
    }
}
