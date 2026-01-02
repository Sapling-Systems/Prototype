use sapling_gui::prelude::*;

#[test]
fn test_simple_layouting() {
  struct RootComponent;

  impl Component for RootComponent {
    fn construct(&self, context: &mut ElementContext) {
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

    fn render(&self, layout: &ResolvedLayout) {
      println!("RootComponent");
      assert_eq!(layout.height, 600.0);
      assert_eq!(layout.width, 400.0);
      assert_eq!(layout.x, 0.0);
      assert_eq!(layout.y, 0.0);
    }
  }

  struct ChildComponent;

  impl Component for ChildComponent {
    fn construct(&self, _context: &mut ElementContext) {}

    fn render(&self, layout: &ResolvedLayout) {
      println!("ChildComponent {:?}", layout);
      assert_eq!(layout.height, 568.0);
      assert_eq!(layout.width, 400.0);
      assert_eq!(layout.x, 0.0);
      assert_eq!(layout.y, 32.0);
    }
  }

  struct ChildComponent2;

  impl Component for ChildComponent2 {
    fn construct(&self, context: &mut ElementContext) {
      EndComponent
        .with_layout(vec![
          ElementConstraints::relative_position(),
          ElementConstraints::fixed_size(32.0, 32.0),
        ])
        .build(context);
    }

    fn render(&self, layout: &ResolvedLayout) {
      println!("ChildComponent2 {:?}", layout);
      assert_eq!(layout.height, 568.0);
      assert_eq!(layout.width, 368.0);
      assert_eq!(layout.x, 32.0);
      assert_eq!(layout.y, 32.0);
    }
  }

  struct EndComponent;

  impl Component for EndComponent {
    fn construct(&self, _context: &mut ElementContext) {}

    fn render(&self, layout: &ResolvedLayout) {
      println!("EndComponent");
      assert_eq!(layout.height, 32.0);
      assert_eq!(layout.width, 32.0);
      assert_eq!(layout.x, 32.0);
      assert_eq!(layout.y, 32.0);
    }
  }

  // Try several times to reduce chance of solver randomness / failure
  for _ in 0..30 {
    let mut orchestrator = Orchestrator::new();
    orchestrator.construct_and_render(RootComponent, 400.0, 600.0);
  }
}
