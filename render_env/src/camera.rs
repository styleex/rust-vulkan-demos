use std::f32;

use cgmath::{Deg, EuclideanSpace, Matrix4, Point3, SquareMatrix, vec3, Vector3};
use cgmath::{Angle, Rad};
use cgmath::InnerSpace;
use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};

pub struct Camera {
    position: Point3<f32>,
    proj: Matrix4<f32>,
    yaw: f32,
    pitch: f32,

    mouse_pressed: bool,
    last_mouse_position: [i32; 2],

    view_dir: Vector3<f32>,
    up_dir: Vector3<f32>,

    viewport: [u32; 2],

    pub near_clip: f32,
    pub far_clip: f32,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            position: Point3::new(0.0, -0.4, 0.0),
            proj: Matrix4::identity(),
            mouse_pressed: false,
            last_mouse_position: [0, 0],
            viewport: [0, 0],
            view_dir: vec3(0.0, 0.0, -1.0),
            up_dir: vec3(0.0, 1.0, 0.0),
            yaw: -90.0,
            pitch: 0.0,
            near_clip: 0.05,
            far_clip: 48.0,
        }
    }

    pub fn set_viewport(&mut self, w: u32, h: u32) {
        self.viewport = [w, h];
        self.proj = cgmath::perspective(
            Rad::from(Deg(45.0)),
            w as f32 / h as f32,
            self.near_clip,
            self.far_clip,
        );
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        return Matrix4::<f32>::look_at_rh(self.position, self.position + self.view_dir, self.up_dir);
    }

    pub fn skybox_view_matrix(&self) -> Matrix4<f32> {
        return Matrix4::<f32>::look_at_rh(Point3::new(0.0, 0.0, 0.0), Point3::from_vec(self.view_dir), self.up_dir);
    }

    pub fn proj_matrix(&self) -> Matrix4<f32> {
        self.proj
    }

    pub fn position(&self) -> Point3<f32> {
        self.position
    }

    pub fn view_dir(&self) -> Vector3<f32> {
        self.view_dir
    }

    fn handle_keyboard(&mut self, input: KeyboardInput) {
        if input.state == ElementState::Pressed {
            match input.virtual_keycode {
                Some(VirtualKeyCode::W) => self.position += self.view_dir * 0.3,
                Some(VirtualKeyCode::S) => self.position -= self.view_dir * 0.3,

                Some(VirtualKeyCode::H) => self.position += self.view_dir * 0.03,
                Some(VirtualKeyCode::J) => self.position -= self.view_dir * 0.03,

                Some(VirtualKeyCode::A) => self.position -= self.view_dir.cross(self.up_dir) * 0.3,
                Some(VirtualKeyCode::D) => self.position += self.view_dir.cross(self.up_dir) * 0.3,
                Some(VirtualKeyCode::Space) => self.position.y += 0.1,
                Some(VirtualKeyCode::LShift) => self.position.y -= 0.1,
                _ => (),
            }
        }
    }

    pub fn mouse_acquired(&self) -> bool {
        self.mouse_pressed
    }

    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        let mut changed = false;
        match event {
            &WindowEvent::KeyboardInput { input, .. } => {
                self.handle_keyboard(input);
                changed = true;
            }

            &WindowEvent::MouseInput { state, button, .. } => {
                self.mouse_pressed = (state == ElementState::Pressed) && (button == MouseButton::Left);
            }
            &WindowEvent::CursorMoved { position, .. } => {
                if !self.mouse_pressed {
                    self.last_mouse_position = position.into();
                    return changed;
                }

                let pos: [i32; 2] = position.into();
                let sensitivity = 0.5;
                let dx = (pos[0] - self.last_mouse_position[0]) as f32 * sensitivity;
                let dy = (pos[1] - self.last_mouse_position[1]) as f32 * sensitivity;
                self.last_mouse_position = position.into();

                self.yaw += dx;
                self.pitch += dy;

                if self.pitch > 89.0 {
                    self.pitch = 89.0;
                }

                if self.pitch < -89.0 {
                    self.pitch = -89.0;
                }

                self.view_dir = Vector3::new(
                    Rad::from(Deg(self.yaw)).cos() * Rad::from(Deg(self.pitch)).cos(),
                    Rad::from(Deg(self.pitch)).sin(),
                    Rad::from(Deg(self.yaw)).sin() * Rad::from(Deg(self.pitch)).cos(),
                ).normalize();

                changed = true
            }
            _ => (),
        }

        changed
    }
}
