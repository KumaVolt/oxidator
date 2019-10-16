use na::Vector3;
use nphysics3d::force_generator::DefaultForceGeneratorSet;
use nphysics3d::joint::DefaultJointConstraintSet;
use nphysics3d::object::{
    BodyPartHandle, ColliderDesc, DefaultBodyHandle, DefaultBodySet, DefaultColliderHandle,
    DefaultColliderSet,
};
use nphysics3d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};

use na::{Isometry3, Matrix3, Matrix4, Point3};
use nalgebra::DMatrix;
use ncollide3d::shape::{Ball, HeightField, ShapeHandle};
use nphysics3d::math::{Inertia, Velocity};
use nphysics3d::object::{BodyStatus, RigidBodyDesc};

pub struct State {
    mechanical_world: DefaultMechanicalWorld<f32>,
    geometrical_world: DefaultGeometricalWorld<f32>,
    bodies: DefaultBodySet<f32>,
    colliders: DefaultColliderSet<f32>,
    joint_constraints: DefaultJointConstraintSet<f32>,
    force_generators: DefaultForceGeneratorSet<f32>,
    balls: Vec<(DefaultBodyHandle, DefaultColliderHandle)>,
}

impl State {
    pub fn new() -> Self {
        let mut mechanical_world = DefaultMechanicalWorld::new(Vector3::new(0.0, -9.81, 0.0));
        let mut geometrical_world = DefaultGeometricalWorld::new();

        let mut bodies = DefaultBodySet::new();
        let mut colliders = DefaultColliderSet::new();
        let mut joint_constraints = DefaultJointConstraintSet::new();
        let mut force_generators = DefaultForceGeneratorSet::new();

        mechanical_world.set_timestep(1.0 / 60.0);
        mechanical_world.counters.enable();
        mechanical_world
            .integration_parameters
            .multiple_ccd_substep_sensor_events_enabled = false;
        mechanical_world
            .integration_parameters
            .max_position_iterations = 1;
        mechanical_world
            .integration_parameters
            .max_velocity_iterations = 1;

        mechanical_world
            .integration_parameters
            .max_ccd_position_iterations = 1;

        mechanical_world.integration_parameters.allowed_linear_error = 0.01;
        mechanical_world
            .integration_parameters
            .allowed_angular_error = 0.01;
        let step = 2048;
        for i in (0..2048 / step).step_by(1) {
            for j in (0..2048 / step).step_by(1) {
                let ground_rb = RigidBodyDesc::new()
                    .translation(Vector3::new(
                        (i * step) as f32 + (step / 2) as f32,
                        0.0,
                        (j * step) as f32 + (step / 2) as f32,
                    ))
                    .status(BodyStatus::Static)
                    .gravity_enabled(false)
                    .build();

                let ground_rb_handle = bodies.insert(ground_rb);

                let heights = DMatrix::from_fn(step, step, |a, b| {
                    crate::heightmap::z((i * step) as f32 + b as f32, (j * step) as f32 + a as f32)
                });
                let mut heightfield: HeightField<f32> =
                    HeightField::new(heights, Vector3::new(step as f32, 1.0, step as f32));

                let shape =
                    ShapeHandle::new(ncollide3d::shape::Plane::new(Vector3::<f32>::y_axis()));

                let shape = ShapeHandle::new(heightfield);
                //    let shape = ShapeHandle::new(Ball::new(1.0));
                let ground_col =
                    ColliderDesc::new(shape).build(BodyPartHandle(ground_rb_handle, 0));
                let ground_col_handle = colliders.insert(ground_col);
            }
        }

        let mut balls = Vec::new();
        for i in (100..2000).step_by(50) {
            for j in (100..2000).step_by(50) {
                let ball_rb = RigidBodyDesc::new()
                    .translation(Vector3::y() * 5.0)
                    .rotation(Vector3::y() * 5.0)
                    .position(Isometry3::new(
                        Vector3::new(i as f32, 100.0, j as f32),
                        Vector3::y() * 3.141592,
                    ))
                    //        .velocity(Velocity::linear(1.0, 2.0, 3.0))
                    .mass(0.1)
                    .gravity_enabled(true)
                    .sleep_threshold(None)
                    .build();

                let ball_rb_handle = bodies.insert(ball_rb);

                //Ball::new(0.7)
                let shape =
                    ShapeHandle::new(ncollide3d::shape::Cuboid::new(Vector3::new(0.5, 0.5, 0.5)));
                let ball_col = ColliderDesc::new(shape)
                    // The collider density. If non-zero the collider's mass and angular inertia will be added
                    // to the inertial properties of the body part it is attached to.
                    // Default: 0.0
                    .density(1.3)
                    .build(BodyPartHandle(ball_rb_handle, 0));

                let ball_col_handle = colliders.insert(ball_col);

                balls.push((ball_rb_handle, ball_col_handle));
            }
        }

        println!("Number of cubes {}", balls.len());

        State {
            mechanical_world,
            geometrical_world,
            bodies,
            colliders,
            joint_constraints,
            force_generators,

            balls,
        }
    }

    pub fn step(&mut self) {
        self.mechanical_world.step(
            &mut self.geometrical_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        );
    }

    pub fn draw_ui(&self, ui: &imgui::Ui) {
        use imgui::*;
        let counters = self.mechanical_world.counters;
        let profile = format!(
            r#"Total: {:.2}ms
Collision detection: {:.2}ms
|_ Broad-phase: {:.2}ms
   Narrow-phase: {:.2}ms
Island computation: {:.2}ms
Solver: {:.2}ms
|_ Assembly: {:.2}ms
   Velocity resolution: {:.2}ms
   Position resolution: {:.2}ms
CCD: {:.2}ms
|_ # of substeps: {}
   TOI computation: {:.2}ms
   Broad-phase: {:.2}ms
   Narrow-phase: {:.2}ms
   Solver: {:.2}ms"#,
            counters.step_time() * 1000.0,
            counters.collision_detection_time() * 1000.0,
            counters.broad_phase_time() * 1000.0,
            counters.narrow_phase_time() * 1000.0,
            counters.island_construction_time() * 1000.0,
            counters.solver_time() * 1000.0,
            counters.assembly_time() * 1000.0,
            counters.velocity_resolution_time() * 1000.0,
            counters.position_resolution_time() * 1000.0,
            counters.ccd_time() * 1000.0,
            counters.ccd.num_substeps,
            counters.ccd.toi_computation_time.time() * 1000.0,
            counters.ccd.broad_phase_time.time() * 1000.0,
            counters.ccd.narrow_phase_time.time() * 1000.0,
            counters.ccd.solver_time.time() * 1000.0
        );

        let edit_height_window = imgui::Window::new(im_str!("Physic statistics"));
        edit_height_window
            .size([400.0, 230.0], imgui::Condition::FirstUseEver)
            .position([3.0, 412.0], imgui::Condition::FirstUseEver)
            .collapsed(false, imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text(im_str!("{}", profile));
            });
    }

    pub fn balls_transform(&self) -> Vec<Matrix4<f32>> {
        self.balls
            .iter()
            .map(|(bh, ch)| {
                let rigid_body = self.bodies.rigid_body(*bh).unwrap();
                rigid_body.position().to_homogeneous()
            })
            .collect()
    }
}