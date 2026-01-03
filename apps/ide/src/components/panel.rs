use sapling_gui::prelude::*;

pub struct Panel;

impl Component for Panel {
  fn construct(&self, context: &mut ElementContext) {
    let _title_bar = StyledView::new()
      .with_background_color(context.theme.color_primary)
      .with_border_radius_even(context.theme.radius_default)
      .with_layout(vec![ElementConstraints::cover_parent()])
      .build(context);

    let _title = FormattedTextView::new()
      .add_text(FontVariant::DefaultForeground, "Viewing".into())
      .add_text(FontVariant::DefaultForegroundBold, "foo".into())
      .add_text(FontVariant::DefaultForeground, "in".into())
      .add_text(
        FontVariant::DefaultForegroundBold,
        "Structural Editor".into(),
      )
      .with_layout(vec![ElementConstraints::cover_parent_padding(
        context.theme.spacing_large,
        context.theme.spacing_default + 2.0,
        0.0,
        0.0,
      )])
      .build(context);

    let _content = StyledView::new()
      .with_background_color(context.theme.color_background_contrast)
      .with_border_radius_even(context.theme.radius_default)
      .with_drop_shadow(context.theme.drop_shadow_default.clone())
      .with_layout(vec![ElementConstraints::cover_parent_padding(
        0.0,
        context.theme.spacing_xlarge,
        0.0,
        0.0,
      )])
      .build(context);
  }
}
