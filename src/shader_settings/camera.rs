use winit::{
    event::*,
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0, // wgpuとOpenGLでz座標が異なるらしい
    0.0, 0.0, 0.5, 1.0, // wはその影響の調整?
);

pub struct Camera {
    pub eye: cgmath::Point3<f32>, // おそらく位置
    pub target: cgmath::Point3<f32>, // ? 視線らしい (have it look at the origin)
    pub up: cgmath::Vector3<f32>, // おそらく目の上の方向 (上の情報あるならいらなくね?)
    pub aspect: f32, // アスペクト(縦横)比
    pub fovy: f32,  // 画角 θ (多分ラジアン)
    // 以下はまるぺけさんに書いてる
    // http://marupeke296.com/DXG_No70_perspective.html
    pub znear: f32,
    pub zfar: f32, 
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // cgmath ライブラリすげぇ...みたいな
        // このあたりはwebglの基本が詳しそう
        // https://webglfundamentals.org/webgl/lessons/ja/webgl-2d-matrices.html
        // view 行列で対象をカメラの前に移動する。
        let view = cgmath::Matrix4::look_at(self.eye, self.target, self.up);
        // 遠近法のための変形行列
        // まるぺけさんのところを見るといい
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

pub struct CameraController {
    pub speed: f32,
    is_up: bool,
    is_down: bool,
    is_forward: bool,
    is_backword: bool,
    is_left: bool,
    is_right: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_up: false,
            is_down: false,
            is_forward: false,
            is_backword: false,
            is_left: false,
            is_right: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::Space => {
                        self.is_up = is_pressed;
                        true
                    },
                    VirtualKeyCode::LShift => {
                        self.is_down = is_pressed;
                        true
                    },
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.is_forward = is_pressed;
                        true
                    },
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left = is_pressed;
                        true
                    },
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backword = is_pressed;
                        true
                    },
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right = is_pressed;
                        true
                    },
                    _ => false,
                }
            },
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        if self.is_forward && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backword {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);
        // 再計算
        let forward = camera.target - camera.eye;
        // let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        if self.is_right {
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}