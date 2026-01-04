use sapling_app::App;
use sapling_gui::{NoopRenderer, prelude::*};

#[test]
fn test_simple_layouting() {
  #[derive(Debug)]
  struct RootComponent;

  impl Component for RootComponent {
    fn construct(&mut self, context: &mut ElementContext) {
      ChildComponent
        .with_layout(vec![ElementConstraints::cover_parent_padding(
          0.0, 32.0, 0.0, 0.0,
        )])
        .with_children(|context| {
          ChildComponent2
            .with_layout(vec![ElementConstraints::cover_parent_padding(
              32.0, 0.0, 0.0, 0.0,
            )])
            .build(context);
        })
        .build(context);
    }

    fn render(&self, context: &mut RenderContext) {
      println!("RootComponent");
      assert_eq!(context.layout.height, 600.0);
      assert_eq!(context.layout.width, 400.0);
      assert_eq!(context.layout.x, 0.0);
      assert_eq!(context.layout.y, 0.0);
    }
  }

  #[derive(Debug)]
  struct ChildComponent;

  impl Component for ChildComponent {
    fn construct(&mut self, _context: &mut ElementContext) {}

    fn render(&self, context: &mut RenderContext) {
      println!("ChildComponent {:?}", context.layout);
      assert_eq!(context.layout.height, 568.0);
      assert_eq!(context.layout.width, 400.0);
      assert_eq!(context.layout.x, 0.0);
      assert_eq!(context.layout.y, 32.0);
    }
  }

  #[derive(Debug)]
  struct ChildComponent2;

  impl Component for ChildComponent2 {
    fn construct(&mut self, context: &mut ElementContext) {
      EndComponent
        .with_layout(vec![
          ElementConstraints::relative_position(),
          ElementConstraints::fixed_size(32.0, 32.0),
        ])
        .build(context);
    }

    fn render(&self, context: &mut RenderContext) {
      println!("ChildComponent2 {:?}", context.layout);
      assert_eq!(context.layout.height, 568.0);
      assert_eq!(context.layout.width, 368.0);
      assert_eq!(context.layout.x, 32.0);
      assert_eq!(context.layout.y, 32.0);
    }
  }

  #[derive(Debug)]
  struct EndComponent;

  impl Component for EndComponent {
    fn render(&self, context: &mut RenderContext) {
      println!("EndComponent");
      assert_eq!(context.layout.height, 32.0);
      assert_eq!(context.layout.width, 32.0);
      assert_eq!(context.layout.x, 32.0);
      assert_eq!(context.layout.y, 32.0);
    }
  }

  // Try several times to reduce chance of solver randomness / failure
  let mut theme = Theme::no_fonts();
  let mut app = App::new(8);
  for _ in 0..30 {
    let mut orchestrator = Orchestrator::new(true);
    orchestrator.construct_and_render(
      RootComponent,
      400.0,
      600.0,
      &mut NoopRenderer,
      &mut theme,
      &mut app,
      &InputState::default(),
    );
  }
}
