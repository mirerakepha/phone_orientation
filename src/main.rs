use minifb::{Key, Window, WindowOptions};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;


/// Window dimensions
const WIDTH: usize = 900;
const HEIGHT: usize = 700;
const CAM_DIST: f32 = 4.0; // camera on the Z-axis
const FOCAL: f32 = 400.0; // bigger
const YAW: f32 = -0.6; // camera orbit Left|Right radians
const PITCH: f32 = 0.5; // camera looks down from above
const GRID_HALF: i32 = 5;
const POS_SCALE: f32 = 10.0;

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self {x, y, z}
    }
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    fn scale(self, s: f32) -> Vec3{
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
    fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        Vec3::new(
            self.x + (o.x - self.x) * t,
            self.y + (o.y - self.y) * t,
            self.z + (o.z - self.z) * t,
        )
    }
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
}
// unit quantenion w + xi + yj + zk
#[derive(Clone, Copy)]
struct Quat {
    w: f32,
    x: f32,
    y: f32,
    z: f32,
}

impl Quat {
    // axis must be a unit vector
    fn from_axis_angle(axis: Vec3, angle: f32) -> Quat {
        let (s, c) = (angle / 2.0).sin_cos();
        Quat { w: c, x: axis.x * s, y: axis.y * s, z: axis.z * s }
    }

    // Hamilton product resulr = apply O first then Self 
    fn mul(self, o: Quat) -> Quat {
        Quat {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }

    // Keep it unit length fights floating-point drift
    fn normalize(self) -> Quat {
        let n = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        Quat { w: self.w / n, x: self.x / n, y: self.y / n, z: self.z / n }
    }

    // Rotate a vector: equivalent to q * v * q^-1,
    fn rotate(self, v: Vec3) -> Vec3 {
        let u = Vec3::new(self.x, self.y, self.z);
        let t = u.cross(v).scale(2.0);
        v.add(t.scale(self.w)).add(u.cross(t))
    }

    fn identity() -> Quat {
        Quat { w: 1.0, x: 0.0, y: 0.0, z: 0.0 }
    }

    fn nlerp(self, o: Quat, t: f32) -> Quat {
        let dot = self.w * o.w + self.x * o.x + self.y * o.y + self.z * o.z;
        let s = if dot < 0.0 { -1.0 } else { 1.0 };
        Quat {
            w: self.w + (s * o.w - self.w) * t,
            x: self.x + (s * o.x - self.x) * t,
            y: self.y + (s * o.y - self.y) * t,
            z: self.z + (s * o.z - self.z) * t,
        }
        .normalize()
    }
}

// fixed camera orientation rotate the world around x and y
fn view(p: Vec3) -> Vec3 {
    let (sy, cy) = YAW.sin_cos();
    let x1 = p.x * cy + p.z * sy;
    let z1 = -p.x * sy + p.z * cy;

    let (sp, cp) = PITCH.sin_cos();
    let y2 = p.y * cp - z1 * sp;
    let z2 = p.y * sp + z1 * cp;

    Vec3::new(x1, y2, z2)
}

// Perspective projection 3D onto a 2D screen
fn project(p: Vec3) -> Option<(i32, i32)> {
    let depth = CAM_DIST - p.z;
    if depth < 0.1 {
        return None;
    }
    let sx = WIDTH as f32 / 2.0 + p.x * FOCAL / depth;
    let sy = HEIGHT as f32 / 2.0 - p.y * FOCAL / depth; // on flip screen Y points down
    Some((sx.round() as i32, sy.round() as i32))
}

fn put_pixel(buf: &mut [u32], x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && (x as usize) < WIDTH && (y as usize) < HEIGHT {
        buf [y as usize * WIDTH + x as usize] = color;
    }

}

// BRESENHAM LINE ALGORITHM
fn draw_line(
    buf: &mut [u32],
    (mut x0, mut y0): (i32, i32),
    (x1, y1): (i32, i32),
    color: u32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 {1} else {-1};
    let sy = if y0 < y1 {1} else {-1};
    let mut err = dx + dy;

    loop {
        put_pixel(buf, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

// draw line btwn 2 3D points
fn line3d(buf: &mut [u32], a: Vec3, b: Vec3, color: u32) {
    if let (Some(pa), Some(pb)) = (project(view(a)), project(view(b))) {
        draw_line(buf, pa, pb, color);
    }
}

fn draw_grid(buf: &mut [u32]) {
    let h = GRID_HALF as f32;
    for i in -GRID_HALF..=GRID_HALF {
        let t = i as f32;
        line3d(buf, Vec3::new(t, 0.0, -h), Vec3::new(t, 0.0, h), 0x00303040); // lines along Z
        line3d(buf, Vec3::new(-h, 0.0, t), Vec3::new(h, 0.0, t), 0x00303040); // lines along X
    }
}

//  ---------  PHONE  ---------------
fn edge(a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> i64 {
    (b.0 - a.0) as i64 * (c.1 - a.1) as i64 - (b.1 - a.1) as i64 * (c.0 - a.0) as i64
}

fn fill_tri(buf: &mut [u32], a: (i32, i32), b: (i32, i32), c: (i32, i32), color: u32) {
    let min_x = a.0.min(b.0).min(c.0).max(0);
    let max_x = a.0.max(b.0).max(c.0).min(WIDTH as i32 - 1);
    let min_y = a.1.min(b.1).min(c.1).max(0);
    let max_y = a.1.max(b.1).max(c.1).min(HEIGHT as i32 - 1);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x, y);
            let w0 = edge(b, c, p);
            let w1 = edge(c, a, p);
            let w2 = edge(a, b, p);
            if (w0 >= 0 && w1 >= 0 && w2 >= 0) || (w0 <= 0 && w1 <= 0 && w2 <= 0) {
                buf[y as usize * WIDTH + x as usize] = color;
            }
        }
    }
}

fn shade(c: u32, k: f32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 0xFF) as f32 * k).min(255.0)) as u32;
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

// (corner indices, counter-clockwise seen from outside; local outward normal; base color)
const FACES: [([usize; 4], (f32, f32, f32), u32); 6] = [
    ([4, 5, 7, 6], (0.0, 0.0, 1.0), 0x002A6FDB),  // front (screen)
    ([0, 2, 3, 1], (0.0, 0.0, -1.0), 0x00A0A0A8), // back
    ([1, 3, 7, 5], (1.0, 0.0, 0.0), 0x00C0C0C8),  // right
    ([0, 4, 6, 2], (-1.0, 0.0, 0.0), 0x00C0C0C8), // left
    ([2, 6, 7, 3], (0.0, 1.0, 0.0), 0x00C0C0C8),  // top
    ([0, 1, 5, 4], (0.0, -1.0, 0.0), 0x00C0C0C8), // bottom
];

fn draw_phone(buf: &mut [u32], q: Quat, center: Vec3) {
    let half = Vec3::new(0.35, 0.75, 0.04);
    let light = Vec3::new(0.30, 0.80, 0.52);
    let cam = Vec3::new(0.0, 0.0, CAM_DIST);

    // 8 corners in view space (bit0 -> x, bit1 -> y, bit2 -> z; set bit = +)
    let mut v = [Vec3::new(0.0, 0.0, 0.0); 8];
    for i in 0..8usize {
        let local = Vec3::new(
            if i & 1 != 0 { half.x } else { -half.x },
            if i & 2 != 0 { half.y } else { -half.y },
            if i & 4 != 0 { half.z } else { -half.z },
        );
        v[i] = view(q.rotate(local).add(center));
    }

    for (idx, n, base) in FACES {
        let (a, b, c) = (v[idx[0]], v[idx[1]], v[idx[2]]);

        // Skip faces pointing away from the camera
        if b.sub(a).cross(c.sub(a)).dot(cam.sub(a)) <= 0.0 {
            continue;
        }

        // Brightness from how directly the face points at the light
        let wn = q.rotate(Vec3::new(n.0, n.1, n.2));
        let color = shade(base, 0.35 + 0.65 * wn.dot(light).max(0.0));

        let pts: Option<Vec<(i32, i32)>> = idx.iter().map(|&i| project(v[i])).collect();
        let Some(p) = pts else { continue };
        fill_tri(buf, p[0], p[1], p[2], color);
        fill_tri(buf, p[0], p[2], p[3], color);
    }
}

#[derive(Clone, Copy)]
struct Pose {
    pos: Vec3,
    rot: Quat,
}

impl Pose {
    fn start() -> Pose {
        Pose { pos: Vec3::new(1.5, 1.5, 1.5), rot: Quat::identity() }
    }
}

// "qw qx qy qz px py pz" -> Pose
fn parse_pose(line: &str) -> Option<Pose> {
    let v: Vec<f32> = line
        .split_whitespace()
        .map(|s| s.parse().ok())
        .collect::<Option<Vec<_>>>()?;
    if v.len() != 7 {
        return None;
    }
    Some(Pose {
        rot: Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), -std::f32::consts::FRAC_PI_2)
            .mul(Quat { w: v[0], x: v[1], y: v[2], z: v[3] })
            .normalize(),
        pos: Vec3::new(v[4] * POS_SCALE, v[6] * POS_SCALE + 1.5, -v[5] * POS_SCALE),
    })
}

// listen to localhost:5555
fn spawn_listener(tx: mpsc::Sender<Pose>) {
    thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:8765").expect("Could not bind port 5555");
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            for line in BufReader::new(stream).lines() {
                let Ok(line) = line else { break };
                if let Some(p) = parse_pose(&line) {
                    if tx.send(p).is_err() {
                        return;
                    }
                }
            }
        // client disconnected? loop back and wait for the next one
        }
    });
}

fn main() {

    // pixel buffer
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    // window
    let mut window = Window::new(
        "View Phone Orientation", // window title
        WIDTH, 
        HEIGHT, 
        WindowOptions::default(),
    ).unwrap_or_else(|e| { 
            panic!("Failed to create a window {}", e);
    });

    // 60 fps update limit
    window.set_target_fps(60);


    // Background thread: reads lines from stdin, parses them, sends poses to main
    let (tx, rx) = mpsc::channel::<Pose>();
    spawn_listener(tx);


    let mut target = Pose::start(); // what the keyboard/feed says
    let mut shown = Pose::start();  // what we actually draw (eases toward target)
    const ROT_STEP: f32 = 0.03;
    const MOVE_STEP: f32 = 0.03;
    const SMOOTH: f32 = 0.25; // 0 = frozen, 1 = no smoothing

    // main loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // logic

        buffer.fill(0x00101018); // background color

        draw_grid(&mut buffer);

        // the Axes
        let o = Vec3::new(0.0, 0.0, 0.0);
        line3d(&mut buffer, o, Vec3::new(2.5, 0.0, 0.0), 0x00FF4040); // X red
        line3d(&mut buffer, o, Vec3::new(0.0, 2.5, 0.0), 0x0040FF40); // Y green
        line3d(&mut buffer, o, Vec3::new(0.0, 0.0, 2.5), 0x004080FF); // Z blue
        
        // Turn held keys into a small rotation about the phone's own axes
        let mut delta = Quat::identity();
        let x_axis = Vec3::new(1.0, 0.0, 0.0);
        let y_axis = Vec3::new(0.0, 1.0, 0.0);
        let z_axis = Vec3::new(0.0, 0.0, 1.0);

        if window.is_key_down(Key::W) { delta = delta.mul(Quat::from_axis_angle(x_axis, -ROT_STEP)); }
        if window.is_key_down(Key::S) { delta = delta.mul(Quat::from_axis_angle(x_axis,  ROT_STEP)); }
        if window.is_key_down(Key::A) { delta = delta.mul(Quat::from_axis_angle(y_axis, -ROT_STEP)); }
        if window.is_key_down(Key::D) { delta = delta.mul(Quat::from_axis_angle(y_axis,  ROT_STEP)); }
        if window.is_key_down(Key::Q) { delta = delta.mul(Quat::from_axis_angle(z_axis,  ROT_STEP)); }
        if window.is_key_down(Key::E) { delta = delta.mul(Quat::from_axis_angle(z_axis, -ROT_STEP)); }
        target.rot = target.rot.mul(delta).normalize();

        // --- Keyboard translation (world axes) ---
        let mut mv = Vec3::new(0.0, 0.0, 0.0);
        if window.is_key_down(Key::Left)      { mv.x -= MOVE_STEP; }
        if window.is_key_down(Key::Right)     { mv.x += MOVE_STEP; }
        if window.is_key_down(Key::Up)        { mv.z -= MOVE_STEP; } // away from camera
        if window.is_key_down(Key::Down)      { mv.z += MOVE_STEP; } // toward camera
        if window.is_key_down(Key::Space)     { mv.y += MOVE_STEP; }
        if window.is_key_down(Key::LeftShift) { mv.y -= MOVE_STEP; }
        target.pos = target.pos.add(mv);
        
        // Spin around Y, plus a slower tumble around X
        /*
        angle += 0.02;
        let qy = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), angle);
        let qx = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), angle * 0.7);
        let q = qy.mul(qx).normalize();
        */

        // Apply the delta in the phone's local frame, then clean up drift
        //orientation = orientation.mul(delta).normalize();
        
        if window.is_key_pressed(Key::R, minifb::KeyRepeat::No) {
            target = Pose::start();
        }

        // external feed
        while let Ok(p) = rx.try_recv() {
            target = p;
        }

        // Ease what we draw toward the target
        shown.rot = shown.rot.nlerp(target.rot, SMOOTH);
        shown.pos = shown.pos.lerp(target.pos, SMOOTH);


        draw_phone(&mut buffer, shown.rot, shown.pos);
        
        // update window with buffer contents
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }


}
