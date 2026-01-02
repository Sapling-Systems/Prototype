use raylib::prelude::*;
use sapling_app::App;
use sapling_gui::{RaylibRenderer, prelude::*};

fn main() {
  let mut app = App::new(128);

  let (mut rl, thread) = raylib::init()
    .size(1280, 720)
    .title("Sapling IDE")
    .msaa_4x()
    .vsync()
    .resizable()
    .build();

  let mut theme = Theme::new(&mut rl, &thread);

  while !rl.window_should_close() {
    let width = rl.get_render_width();
    let height = rl.get_render_height();

    let mut d = rl.begin_drawing(&thread);

    d.clear_background(Color::from_hex("212121").unwrap());

    let mut orchestrator = Orchestrator::new();
    let mut renderer = RaylibRenderer::new(d);
    let (rendering_duration, layouting_duration) = orchestrator.construct_and_render(
      RootView,
      width as f32,
      height as f32,
      &mut renderer,
      &mut theme,
    );
    let mut d = renderer.end();

    let fps = d.get_fps();
    theme.primary_font.as_mut().unwrap().draw_text(
      &mut d,
      &format!(
        "FPS: {}\nFacts: {}\nRender {:.2}ms\nLayout {:.2}ms",
        fps,
        app.get_raw_database_mut().facts_mut().len(),
        rendering_duration.as_millis() as f32,
        layouting_duration.as_millis() as f32
      ),
      Vector2::new(width as f32 - 200.0, 32.0),
      18.0,
      Color::RED,
    );
  }
}

struct RootView;

impl Component for RootView {
  fn construct(&self, context: &mut ElementContext) {
    StyledView::new()
      .with_background_color(Color::RED)
      .with_border(4.0, Color::WHITE)
      .with_border_radius_even(16.0)
      .with_layout(vec![ElementConstraints::cover_parent_even_padding(128.0)])
      .with_children(|context| {
        TextView::new(FontVariant::Primary, "Hello World!".into())
          .with_horizontal_alignment(TextHorizontalAlignment::Center)
          .with_layout(vec![ElementConstraints::cover_parent_even_padding(32.0)])
          .build(context);

        StyledView::new()
          .with_background_color(Color::BLUE)
          .with_border(1.0, Color::WHITE)
          .with_border_radius_even(8.0)
          .with_layout(vec![
            ElementConstraints::center(),
            ElementConstraints::fixed_size(64.0, 64.0),
          ])
          .build(context);
      })
      .build(context);
  }
}
