use raylib::prelude::*;
use sapling_app::App;
use sapling_data_model::SubjectSelector;
use sapling_gui::{DebuggerView, RaylibRenderer, RaylibRendererState, prelude::*};

use crate::{
  components::{
    loc::LinesOfCodeView,
    panel::PanelView,
    structure_editor::{StructureEditor, data::SubjectFactCollection},
  },
  demo::insert_demo_data,
};

mod components;
mod demo;

fn main() {
  let mut app = App::new(128);
  insert_demo_data(&mut app);

  let (mut rl, thread) = raylib::init()
    .size(1280, 720)
    .title("Sapling IDE")
    .vsync()
    .resizable()
    .build();

  let mut theme = Theme::new(&mut rl, &thread);
  let mut renderer_state = RaylibRendererState::new(&mut rl, &thread);
  let mut orchestrator = Orchestrator::new(true);

  while !rl.window_should_close() {
    let width = rl.get_render_width();
    let height = rl.get_render_height();

    let input_state = InputState::from_raylib(&mut rl);

    let mut d = rl.begin_drawing(&thread);

    d.clear_background(theme.color_background);

    let mut renderer = RaylibRenderer::new(d, &mut renderer_state, thread.clone());
    let (rendering_duration, layouting_duration) = orchestrator.construct_and_render(
      RootView,
      width as f32,
      height as f32,
      &mut renderer,
      &mut theme,
      &mut app,
      &input_state,
    );
    let mut d = renderer.end();

    let fps = d.get_fps();
    theme.font_primary.as_mut().unwrap().draw_text(
      &mut d,
      &format!(
        "FPS: {}\nFacts: {}\nRender {:.2}ms\nLayout {:.2}ms",
        fps,
        app.get_raw_database_mut().facts_mut().len(),
        rendering_duration.as_millis() as f32,
        layouting_duration.as_millis() as f32
      ),
      Vector2::new(width as f32 - 200.0, height as f32 - 128.0),
      18.0,
      Color::RED,
    );
  }
}

#[derive(Debug)]
struct RootView;

impl Component for RootView {
  fn construct(&mut self, context: &mut ElementContext) {
    PanelView::new()
      .with_content(|context| {
        let loc_view = LinesOfCodeView::new(vec![0; 10], 2)
          .with_layout(vec![ElementConstraints::cover_parent_even_padding(
            context.theme.spacing_large,
          )])
          .build(context);

        let person1 = context.app.get_global_by_name("Person 1").unwrap();
        let subject_fact_collection = SubjectFactCollection::new(
          SubjectSelector {
            subject: person1,
            evaluated: false,
            property: None,
          },
          context.app,
        );

        StructureEditor::new(subject_fact_collection)
          .with_layout(vec![
            ElementConstraints::anchor_to_right_of(loc_view, context.theme.spacing_xlarge),
            ElementConstraints::relative_top(context.theme.spacing_large),
          ])
          .build(context);
      })
      .with_layout(vec![
        ElementConstraints::relative_top(32.0),
        ElementConstraints::relative_left(32.0),
        ElementConstraints::fixed_size(500.0, 500.0),
      ])
      .build(context);

    DebuggerView::new().build(context);
  }
}
