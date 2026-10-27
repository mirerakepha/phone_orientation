use minifb::{Key, Window, WindowOptions};

/// Window dimensions
const WIDTH: usize = 900;
const HEIGHT: usize = 700;
const CAM_DIST: f32 = 4.0; // camera on the Z-axis
const FOCAL: f32 = 400.0; // bigger
const YAW: f32 = 0.6; // camera orbit Left|Right radians
const PITCH: f32 = 0.5; // camera looks down from above
const GRID_HALF: i32 = 5;

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

// PHONE
fn draw_phone(buf: &mut [u32], q: Quat, center: Vec3) {
    let half = Vec3::new(0.35, 0.75, 0.04);

    let corner = |i: usize| {
        let local = Vec3::new(
            if i & 1 != 0 { half.x } else { -half.x },
            if i & 2 != 0 { half.y } else { -half.y },
            if i & 4 != 0 { half.z } else { -half.z },
        );
        q.rotate(local).add(center)
    };
    for i in 0..8usize {
        for bit in [1usize, 2, 4] {
            if i & bit == 0 {
                let j = i | bit;
                // Screen face (+Z side) is bright, the rest is dim
                let on_screen_face = bit != 4 && i & 4 != 0;
                let color = if on_screen_face { 0x00FFFFFF } else { 0x00808090 };
                line3d(buf, corner(i), corner(j), color);
            } 
        }
    }
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

    let mut angle: f32 = 0.0;


    // main loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // logic

        buffer.fill(0x00101018); // background color

        draw_grid(&mut buffer);

        // the Axes
        let o = Vec3::new(0.0, 0.0, 0.0);
        line3d(&mut buffer, o, Vec3::new(1.5, 0.0, 0.0), 0x00FF4040); // X red
        line3d(&mut buffer, o, Vec3::new(0.0, 1.5, 0.0), 0x0040FF40); // Y green
        line3d(&mut buffer, o, Vec3::new(0.0, 0.0, 1.5), 0x004080FF); // Z blue
                                                                      //
        // Spin around Y, plus a slower tumble around X
        angle += 0.02;
        let qy = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), angle);
        let qx = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), angle * 0.7);
        let q = qy.mul(qx).normalize();

        draw_phone(&mut buffer, q, Vec3::new(0.0, 1.5, 0.0));
        
        // update window with buffer contents
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }


}
