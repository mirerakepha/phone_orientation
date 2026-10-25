use minifb::{Key, Window, WindowOptions};

/// Window dimensions
const WIDTH: usize = 1280;
const HEIGHT: usize = 640;


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
        for i in buffer.iter_mut() {
            *i = 0xFF0000;
        }

        // update window with buffer contents
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }


}
