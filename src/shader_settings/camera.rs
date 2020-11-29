use cgmath::*;
use winit::event::*;
use winit::dpi::PhysicalPosition;
use std::time::Duration;
use std::f32::consts::FRAC_PI_2;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0, // wgpuとOpenGLでz座標が異なるらしい
    0.0, 0.0, 0.5, 1.0, // wはその影響の調整?
);

#[derive(Debug)]
pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

impl Camera {
    pub fn new<V, Y, P>(
        position: V,
        yaw: Y,
        pitch: P,
    ) -> Self
    where
        V: Into<Point3<f32>>,
        Y: Into<Rad<f32>>,
        P: Into<Rad<f32>>,
    {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(
            self.position, // eye
            Vector3::new(
                self.yaw.0.cos(),
                self.pitch.0.sin(),
                self.yaw.0.sin(),
            ).normalize(), // dir
            Vector3::unit_y(),
        )
    }
}

#[derive(Clone, Copy)]
pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>, // 視野角
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(
            self.fovy,
            self.aspect,
            self.znear,
            self.zfar,
        )
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        use VirtualKeyCode as VKC;
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };
        match key {
            VKC::W | VKC::Up => {
                self.amount_forward = amount;
                true
            }
            VKC::S | VKC::Down => {
                self.amount_backward = amount;
                true
            }
            VKC::A | VKC::Left => {
                self.amount_left = amount;
                true
            }
            VKC::D | VKC::Right => {
                self.amount_right = amount;
                true
            }
            VKC::Space => {
                self.amount_up = amount;
                true
            }
            VKC::LShift => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // assume a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition {
                y: scroll,
                ..
            }) => *scroll as f32, // scrollは借用状態
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // 前後左右
        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        // ここの数式は数学。どうしてこうなのか確認してない
        // ここが参考になりそう。https://watako-lab.com/2019/01/23/roll_pitch_yaw/
        // ヨーから現在のカメラの前方と右方を算出している模様
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();

        // ここはまぁわかるだろう
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // ズーム
        // カメラを移動させることで疑似的にズームしてる
        // ヨーだけではなくピッチでカメラの向いている方向に移動させる
        let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
        let scrollward = Vector3::new(
            pitch_cos * yaw_cos,
            pitch_sin,
            pitch_cos * yaw_sin,
        ).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // 上下
        // ロールしないのでy軸をずらすだけでいい
        camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

        // 回転
        camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // verticalな方向のカメラワーク(上下に動かす)には上下限がある。
        if camera.pitch < -Rad(FRAC_PI_2) { // -π/2 より小さい
            camera.pitch = -Rad(FRAC_PI_2);
        } else if camera.pitch > Rad(FRAC_PI_2) {
            camera.pitch = Rad(FRAC_PI_2);
        }
    }
}

pub struct CameraSetting {
    pub camera: Camera,
    pub projection: Projection,
    pub camera_controller: CameraController,
    pub last_mouse_pos: PhysicalPosition<f64>,
    pub mouse_pressed: bool,
}

impl CameraSetting {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            camera: Camera::new(
                (0.0, 5.0, 10.0),
                Deg(-90.0),
                Deg(-20.0),
            ),
            projection: Projection::new(width, height, Deg(45.0), 0.1, 100.0),
            camera_controller: CameraController::new(4.0, 0.4),
            last_mouse_pos: (0.0, 0.0).into(),
            mouse_pressed: false,
        }
    }
}