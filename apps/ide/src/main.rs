use raylib::prelude::*;
use sapling_app::App;

use crate::font::Font;

mod font;

fn main() {
  let mut app = App::new(128);

  let (mut rl, thread) = raylib::init()
    .size(1280, 720)
    .title("Sapling IDE")
    .msaa_4x()
    .vsync()
    .build();

  let mut font_medium = Font::new(
    &mut rl,
    &thread,
    "./apps/ide/assets/fonts/FiraCode-Medium.ttf",
  )
  .unwrap();

  while !rl.window_should_close() {
    let width = rl.get_render_width();
    let _height = rl.get_render_height();

    let mut d = rl.begin_drawing(&thread);

    d.clear_background(Color::from_hex("212121").unwrap());
    let fps = d.get_fps();
    font_medium.draw_text(
      &mut d,
      &format!(
        "FPS: {}\nFacts: {}",
        fps,
        app.get_raw_database_mut().facts_mut().len()
      ),
      Vector2::new(width as f32 - 200.0, 32.0),
      24.0,
      Color::RED,
    );
    font_medium.draw_text(
      &mut d,
      "Ut nobis harum inventore recusandae. Et quia sunt perferendis non inventore officia.",
      Vector2::new(40.0, 40.0),
      24.0,
      Color::BLACK,
    );
  }
}
