use std::{
    path::PathBuf,
};

use piston_window::{
    OpenGL,
    PistonWindow,
    WindowSettings,
    TextureSettings,
    Glyphs,
};

const SCREEN_WIDTH: u32 = 640;
const SCREEN_HEIGHT: u32 = 480;

fn main() {
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("periodogram", [SCREEN_WIDTH, SCREEN_HEIGHT])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    let mut font_path = PathBuf::from("assets");
    font_path.push("FiraSans-Regular.ttf");
    let _glyphs = Glyphs::new(&font_path, window.create_texture_context(), TextureSettings::new()).unwrap();

    todo!();
}
