use sapling_gui::prelude::*;

#[derive(Debug)]
pub struct LinesOfCodeView {
  max_digits: usize,
  lines: Vec<usize>,
  line_height: f32,
}

impl LinesOfCodeView {
  pub fn new(lines: Vec<usize>, max_digits: usize) -> Self {
    Self {
      lines,
      max_digits,
      line_height: 1.3,
    }
  }
}

impl Component for LinesOfCodeView {
  fn construct(&mut self, context: &mut ElementContext) {
    let font_config = context.theme.text_config(FontVariant::Primary);
    let max_digit_string = format!("{:0width$}", 0, width = self.max_digits);
    let text_size = font_config
      .font
      .calculate_text_size(&max_digit_string, font_config.size);

    let total_loc_height: f32 = self
      .lines
      .iter()
      .map(|extra| text_size.x * self.line_height + *extra as f32)
      .sum();

    let total_width = text_size.x + context.theme.spacing_default * 2.0;

    StyledView::new()
      .with_background_color(context.theme.color_background_highlight)
      .with_border_radius_even(16.0)
      .with_layout(vec![
        ElementConstraints::relative_left(0.0),
        ElementConstraints::relative_top(0.0),
        ElementConstraints::fixed_size(
          total_width,
          total_loc_height + context.theme.spacing_default * 2.0,
        ),
      ])
      .build(context);

    let mut y = context.theme.spacing_default;
    for (line, line_extra) in self.lines.iter().enumerate() {
      let text = format!("{:0width$}", line, width = self.max_digits);
      TextView::new(FontVariant::Primary, text)
        .with_horizontal_alignment(TextHorizontalAlignment::Center)
        .with_layout(vec![
          ElementConstraints::relative_left(0.0),
          ElementConstraints::relative_top(y),
          ElementConstraints::fixed_width(total_width),
        ])
        .build(context);

      y += text_size.y * self.line_height + *line_extra as f32;
    }
  }
}
