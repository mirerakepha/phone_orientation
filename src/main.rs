use minifb::{Key, Window, WindowOptions};

/// Window dimensions
const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const CAM_DIST: f32 = 4.0; // camera on the Z-axis
const FOCAL: f32 = 400.0; // bigger
                          //

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

// Perspective projection 3D onto a 2D screen
fn project(p: Vec3) -> (i32, i32) {
    let depth = CAM_DIST - p.z;
    let sx = WIDTH as f32 / 2.0 + p.x * FOCAL / depth;
    let sy = HEIGHT as f32 / 2.0 - p.y * FOCAL / depth; // on flip screen Y points down
    (sx.round() as i32, sy.round() as i32)
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

        // the Axes
        let origin = project(Vec3::new(0.0, 0.0, 0.0));
        draw_line(&mut buffer, origin, project(Vec3::new(1.5, 0.0, 0.0)), 0x00FF4040); // X red
        draw_line(&mut buffer, origin, project(Vec3::new(0.0, 1.5, 0.0)), 0x0040FF40); // Y green
        draw_line(&mut buffer, origin, project(Vec3::new(0.0, 0.0, 1.5)), 0x004080FF); // Z blue
        
        // update window with buffer contents
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }


}
