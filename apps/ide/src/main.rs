use raylib::prelude::*;
use sapling_app::App;
use sapling_data_model::{Query, SubjectSelector};
use sapling_gui::{DebuggerView, RaylibRenderer, RaylibRendererState, prelude::*};

use crate::{
  components::{
    loc::LinesOfCodeView,
    panel::PanelView,
    structure_editor::{StructureEditor, SubjectCollectionView, data::SubjectFactCollection},
  },
  demo::insert_demo_data,
  input::Action,
};

mod components;
mod demo;
mod input;

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

  let mut action_map = ActionMap::default();
  action_map.add_action(Action::EditorSelectModeLeft, KeyboardKey::KEY_H);
  action_map.add_action(Action::EditorSelectModeRight, KeyboardKey::KEY_L);
  action_map.add_action(Action::EditorSelectModeUp, KeyboardKey::KEY_K);
  action_map.add_action(Action::EditorSelectModeDown, KeyboardKey::KEY_J);

  while !rl.window_should_close() {
    let width = rl.get_render_width();
    let height = rl.get_render_height();

    let input_state = InputState::from_raylib(&mut rl, action_map.clone());

    let mut d = rl.begin_drawing(&thread);

    d.clear_background(theme.color_background);

    let mut renderer = RaylibRenderer::new(d, &mut renderer_state, thread.clone());
    let ui_stats = orchestrator.construct_and_render(
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
        "FPS: {}\nFacts: {}\nElements: {} ({})\nConstruct: {:.2}ms\nLayout {:.2}ms\nRender {:.2}ms\n",
        fps,
        app.get_raw_database_mut().facts_mut().len(),
        ui_stats.element_count,
        ui_stats.constrain_count,
        ui_stats.construction_duration.as_millis() as f32,
        ui_stats.layout_duration.as_millis() as f32,
        ui_stats.render_duration.as_millis() as f32,
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
    let person1 = context.app.get_global_by_name("Person 1").unwrap();
    StructureEditor::new(Query {
      evaluated: false,
      meta: None,
      property: None,
      subject: person1,
    })
    .build(context);

    DebuggerView::new().build(context);
  }
}
