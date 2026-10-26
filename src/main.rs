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
        
        // update window with buffer contents
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }


}
