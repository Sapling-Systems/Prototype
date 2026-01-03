use raylib::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

#[derive(Clone, Copy, Debug)]
pub struct CornerRadii {
  pub tl: f32,
  pub tr: f32,
  pub br: f32,
  pub bl: f32,
}

/// Draw a rounded rectangle with independent corner radii.
/// - If `fill` is true, draws a filled shape.
/// - If `outline_thickness` > 0.0, draws an outline with that thickness.
/// - `arc_segments` controls smoothness per corner arc (e.g. 6..24).
pub fn draw_round_rect_per_corner<T: RaylibDraw>(
  d: &mut T,
  rect: Rectangle,
  mut r: CornerRadii,
  arc_segments: i32,
  fill: bool,
  outline_thickness: f32,
  color: Color,
) {
  if rect.width <= 0.0 || rect.height <= 0.0 {
    return;
  }
  let arc_segments = arc_segments.max(1);

  // Clamp radii so they don't exceed half extents and don't overlap on edges.
  clamp_corner_radii(&rect, &mut r);

  // Build perimeter points (clockwise).
  let pts = rounded_rect_points(rect, r, arc_segments);

  if pts.len() < 3 {
    return;
  }

  // Filled: triangle fan from centroid (shape is convex).
  if fill {
    fill_convex_polygon_fan(d, &pts, color);
  }

  // Outline: thick polyline.
  if outline_thickness > 0.0 {
    for i in 0..pts.len() {
      let a = pts[i];
      let b = pts[(i + 1) % pts.len()];
      d.draw_line_ex(a, b, outline_thickness, color);
    }
  }
}

/// Produces a clockwise polyline around the rounded rectangle perimeter.
fn rounded_rect_points(rect: Rectangle, r: CornerRadii, arc_segments: i32) -> Vec<Vector2> {
  let x = rect.x;
  let y = rect.y;
  let w = rect.width;
  let h = rect.height;

  let left = x;
  let right = x + w;
  let top = y;
  let bottom = y + h;

  // Corner centers
  let ctl = Vector2::new(left + r.tl, top + r.tl);
  let ctr = Vector2::new(right - r.tr, top + r.tr);
  let cbr = Vector2::new(right - r.br, bottom - r.br);
  let cbl = Vector2::new(left + r.bl, bottom - r.bl);

  let mut pts = Vec::new();

  // Helper: append a corner arc (clockwise).
  // Angles are in radians.
  let mut arc = |center: Vector2, radius: f32, start: f32, end: f32| {
    if radius <= 0.0 {
      // No arc: just add the corner point implied by start angle.
      pts.push(Vector2::new(
        center.x + radius * start.cos(),
        center.y + radius * start.sin(),
      ));
      return;
    }

    // For clockwise arcs, we want decreasing angle progression if end < start, but
    // here we pass values already set for clockwise quarter-circles.
    for i in 0..=arc_segments {
      let t = i as f32 / arc_segments as f32;
      let ang = start + (end - start) * t;
      pts.push(Vector2::new(
        center.x + radius * ang.cos(),
        center.y + radius * ang.sin(),
      ));
    }
  };

  // Clockwise path starting at top edge (after TL arc), going right.

  // Top-left arc: from 180° to 270° (PI to 3PI/2)
  arc(ctl, r.tl, PI, PI + FRAC_PI_2);

  // Top-right arc: 270° to 360° (3PI/2 to 2PI)
  arc(ctr, r.tr, PI + FRAC_PI_2, 2.0 * PI);

  // Bottom-right arc: 0° to 90°
  arc(cbr, r.br, 0.0, FRAC_PI_2);

  // Bottom-left arc: 90° to 180°
  arc(cbl, r.bl, FRAC_PI_2, PI);

  // The arcs include endpoints, which can create duplicates where arcs meet.
  // Remove near-duplicates to keep outlines clean.
  dedup_near(&mut pts, 0.01);

  pts
}

/// Clamps radii so:
/// - each <= min(w,h)/2
/// - adjacent radii on each side don't sum beyond side length
fn clamp_corner_radii(rect: &Rectangle, r: &mut CornerRadii) {
  let w = rect.width.max(0.0);
  let h = rect.height.max(0.0);
  let max_r = 0.5 * w.min(h);

  r.tl = r.tl.max(0.0).min(max_r);
  r.tr = r.tr.max(0.0).min(max_r);
  r.br = r.br.max(0.0).min(max_r);
  r.bl = r.bl.max(0.0).min(max_r);

  // Ensure sums on each edge fit:
  // top: tl + tr <= w
  // bottom: bl + br <= w
  // left: tl + bl <= h
  // right: tr + br <= h
  scale_pair_to_limit(&mut r.tl, &mut r.tr, w);
  scale_pair_to_limit(&mut r.bl, &mut r.br, w);
  scale_pair_to_limit(&mut r.tl, &mut r.bl, h);
  scale_pair_to_limit(&mut r.tr, &mut r.br, h);
}

fn scale_pair_to_limit(a: &mut f32, b: &mut f32, limit: f32) {
  let sum = *a + *b;
  if sum > limit && sum > 0.0 {
    let s = limit / sum;
    *a *= s;
    *b *= s;
  }
}

fn polygon_centroid(pts: &[Vector2]) -> Vector2 {
  // For a convex polygon (our case), a simple average is fine and stable.
  let mut c = Vector2::new(0.0, 0.0);
  for p in pts {
    c.x += p.x;
    c.y += p.y;
  }
  let inv = 1.0 / (pts.len() as f32);
  c.x *= inv;
  c.y *= inv;
  c
}

fn dedup_near(pts: &mut Vec<Vector2>, eps: f32) {
  if pts.is_empty() {
    return;
  }
  let mut out = Vec::with_capacity(pts.len());
  out.push(pts[0]);

  for p in pts.iter().skip(1) {
    let last = *out.last().unwrap();
    if (p.x - last.x).abs() > eps || (p.y - last.y).abs() > eps {
      out.push(*p);
    }
  }

  // Also avoid a duplicate closing point (if any)
  if out.len() >= 2 {
    let first = out[0];
    let last = *out.last().unwrap();
    if (first.x - last.x).abs() <= eps && (first.y - last.y).abs() <= eps {
      out.pop();
    }
  }

  *pts = out;
}

fn polygon_signed_area(pts: &[Vector2]) -> f32 {
  let mut a = 0.0f32;
  for i in 0..pts.len() {
    let p = pts[i];
    let q = pts[(i + 1) % pts.len()];
    a += p.x * q.y - q.x * p.y;
  }
  0.5 * a
}

fn ensure_ccw(pts: &mut Vec<Vector2>) {
  // In screen coords (y down), the sign can appear flipped depending on convention,
  // but the important part is consistency: if it’s "wrong" for your pipeline, reverse.
  // For raylib default front-face (CCW), we want positive area in typical math coords.
  // Empirically: if your triangles are invisible, reverse and they show.
  if polygon_signed_area(pts) < 0.0 {
    pts.reverse();
  }
}

fn fill_convex_polygon_fan<T: RaylibDraw>(d: &mut T, pts: &[Vector2], color: Color) {
  if pts.len() < 3 {
    return;
  }

  // Fan anchored at vertex 0: (0, i, i+1)
  for i in 1..(pts.len() - 1) {
    let a = pts[0];
    let b = pts[i];
    let c = pts[i + 1];

    // Draw both windings to be immune to whichever front-face is active.
    d.draw_triangle(a, b, c, color);
    d.draw_triangle(a, c, b, color);
  }
}
