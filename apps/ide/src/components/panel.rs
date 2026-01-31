use sapling_gui::prelude::*;

pub struct PanelView {
  content: ChildrenProperty,
}

impl std::fmt::Debug for PanelView {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PanelView").finish()
  }
}

impl PanelView {
  pub fn new() -> Self {
    Self { content: None }
  }

  pub fn with_content<F: FnOnce(&mut ElementContext) + 'static>(mut self, factory: F) -> Self {
    self.content = Some(Box::new(factory));
    self
  }
}

impl Component for PanelView {
  fn construct(&mut self, context: &mut ElementContext) {
    let _title_bar = StyledView::new()
      .with_background_color(context.theme.color_primary)
      .with_border_radius_even(context.theme.radius_default)
      .with_layout(vec![UserElementConstraints::cover_parent(0.0, 0.0)])
      .build(context);

    let _title = FormattedTextView::new()
      .add_text(FontVariant::DefaultForeground, "Viewing".into())
      .add_text(FontVariant::DefaultForegroundBold, "foo".into())
      .add_text(FontVariant::DefaultForeground, "in".into())
      .add_text(
        FontVariant::DefaultForegroundBold,
        "Structural Editor".into(),
      )
      .with_layout(vec![UserElementConstraints::relative_to_parent(
        context.theme.spacing_large,
        context.theme.spacing_default + 2.0,
      )])
      .build(context);

    let content_children = self.content.take().unwrap();
    let _content = StyledView::new()
      .with_background_color(context.theme.color_background_contrast)
      .with_border_radius_even(context.theme.radius_default)
      .with_drop_shadow(context.theme.drop_shadow_default.clone())
      .with_layout(vec![
        UserElementConstraints::relative_to_parent(0.0, context.theme.spacing_xlarge),
        UserElementConstraints::fixed_size(500.0, 300.0),
      ])
      .with_children(|context| {
        content_children(context);
      })
      .build(context);
  }

  fn render(&self, context: &mut RenderContext) {
    println!("PanelView Layout: {:?}", context.layout);
  }
}
