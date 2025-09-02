use bitflags::bitflags;
use glam::{Mat4, Vec3};
use sdl3::keyboard::Keycode;

pub struct Camera {
    pub position: Vec3,

    pub pitch: f32,
    pub yaw: f32,

    up: glam::Vec3,
    forward: glam::Vec3,
    right: glam::Vec3,

    pub fov: f32,

    buttons: Buttons,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Vec3::ZERO,
            pitch: 0.0,
            yaw: 0.0,
            fov: 80.0,
            up: Vec3::Z,
            forward: Vec3::Y,
            right: Vec3::X,
            buttons: Buttons::default(),
        }
    }
}

impl Camera {
    pub fn handle_event(&mut self, event: &sdl3::event::Event) {
        let (key, state) = match event {
            sdl3::event::Event::KeyDown {
                keycode: Some(keycode),
                ..
            } => (keycode, true),
            sdl3::event::Event::KeyUp {
                keycode: Some(keycode),
                ..
            } => (keycode, false),
            _ => return,
        };

        match key {
            Keycode::W => self.buttons.set(Buttons::FORWARD, state),
            Keycode::S => self.buttons.set(Buttons::BACK, state),
            Keycode::A => self.buttons.set(Buttons::LEFT, state),
            Keycode::D => self.buttons.set(Buttons::RIGHT, state),
            Keycode::Left => self.buttons.set(Buttons::LOOK_LEFT, state),
            Keycode::Right => self.buttons.set(Buttons::LOOK_RIGHT, state),
            Keycode::Up => self.buttons.set(Buttons::LOOK_UP, state),
            Keycode::Down => self.buttons.set(Buttons::LOOK_DOWN, state),
            Keycode::LShift => self.buttons.set(Buttons::SPEED, state),
            Keycode::Space => self.buttons.set(Buttons::LUDICROUS_SPEED, state),
            _ => {}
        }
    }

    fn update_vectors(&mut self) {
        self.forward.x = self.pitch.to_radians().cos() * self.yaw.to_radians().sin();
        self.forward.y = self.pitch.to_radians().cos() * self.yaw.to_radians().cos();
        self.forward.z = -self.pitch.to_radians().sin();

        self.forward = self.forward.normalize();
        self.right = self.forward.cross(Vec3::Z).normalize();
        self.up = self.right.cross(self.forward).normalize();
    }

    pub fn update(&mut self, dt: f32) {
        let mut speed = 256.0 * dt;
        if self.buttons.contains(Buttons::SPEED) {
            speed *= 2.0;
        }
        if self.buttons.contains(Buttons::LUDICROUS_SPEED) {
            speed *= 8.0;
        }

        if self.buttons.contains(Buttons::FORWARD) {
            self.position += self.forward * speed;
        }
        if self.buttons.contains(Buttons::BACK) {
            self.position -= self.forward * speed;
        }
        if self.buttons.contains(Buttons::LEFT) {
            self.position -= self.right * speed;
        }
        if self.buttons.contains(Buttons::RIGHT) {
            self.position += self.right * speed;
        }

        let look_speed = 85.0 * dt;
        if self.buttons.contains(Buttons::LOOK_LEFT) {
            self.yaw -= look_speed;
        }
        if self.buttons.contains(Buttons::LOOK_RIGHT) {
            self.yaw += look_speed;
        }
        if self.buttons.contains(Buttons::LOOK_UP) {
            self.pitch -= look_speed;
        }
        if self.buttons.contains(Buttons::LOOK_DOWN) {
            self.pitch += look_speed;
        }
        self.update_vectors();
    }

    pub fn world_to_view(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.forward, self.up)
    }

    pub fn view_to_projective(&self, aspect_ratio: f32) -> Mat4 {
        let near = 0.1;

        Mat4::perspective_infinite_reverse_rh(self.fov.to_radians(), aspect_ratio, near)
    }

    pub fn world_to_projective(&self, aspect_ratio: f32) -> Mat4 {
        self.view_to_projective(aspect_ratio) * self.world_to_view()
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Buttons : u32 {
        const FORWARD = (1 << 0);
        const BACK = (1 << 1);
        const LEFT = (1 << 2);
        const RIGHT = (1 << 3);

        const LOOK_LEFT = (1 << 4);
        const LOOK_RIGHT = (1 << 5);
        const LOOK_UP = (1 << 6);
        const LOOK_DOWN = (1 << 7);

        const SPEED = (1 << 8);
        const LUDICROUS_SPEED = (1 << 9);
    }
}
