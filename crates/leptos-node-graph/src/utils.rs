use crate::types::Position;

pub fn bezier_control_points(start: Position, end: Position) -> (Position, Position) {
    let dx = (end.x - start.x).abs();
    let offset = dx.max(50.0) * 0.5;
    let cp1 = Position::new(start.x + offset, start.y);
    let cp2 = Position::new(end.x - offset, end.y);
    (cp1, cp2)
}

pub fn bezier_path(start: Position, end: Position) -> String {
    let (cp1, cp2) = bezier_control_points(start, end);
    format!(
        "M {},{} C {},{} {},{} {},{}",
        start.x, start.y, cp1.x, cp1.y, cp2.x, cp2.y, end.x, end.y,
    )
}

/// Subway-style orthogonal routing: exit the source horizontally, turn at the
/// mid-x, run vertically to the target's row, then enter the target horizontally —
/// with small rounded corners (quadratic turns). Reads as right-angle "subway map"
/// wiring rather than bezier curves.
pub fn orthogonal_path(start: Position, end: Position) -> String {
    let mid_x = (start.x + end.x) / 2.0;
    // Corner radius, clamped so it never exceeds half the vertical run or either leg.
    let dy = (end.y - start.y).abs();
    let leg_x = (mid_x - start.x).abs().min((end.x - mid_x).abs());
    let r = 8.0_f64.min(dy / 2.0).min(leg_x).max(0.0);
    if r < 0.5 {
        // Degenerate (near-straight / no room to round) — a plain elbow.
        return format!(
            "M {},{} L {},{} L {},{} L {},{}",
            start.x, start.y, mid_x, start.y, mid_x, end.y, end.x, end.y,
        );
    }
    let down = end.y >= start.y;
    let vy0 = if down { start.y + r } else { start.y - r };
    let vy1 = if down { end.y - r } else { end.y + r };
    format!(
        "M {sx},{sy} L {c0x},{sy} Q {mx},{sy} {mx},{vy0} L {mx},{vy1} Q {mx},{ey} {c1x},{ey} L {ex},{ey}",
        sx = start.x,
        sy = start.y,
        c0x = mid_x - r,
        mx = mid_x,
        vy0 = vy0,
        vy1 = vy1,
        ey = end.y,
        c1x = mid_x + r,
        ex = end.x,
    )
}

pub fn bezier_point(start: Position, end: Position, t: f64) -> Position {
    let (cp1, cp2) = bezier_control_points(start, end);
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    Position {
        x: mt3 * start.x + 3.0 * mt2 * t * cp1.x + 3.0 * mt * t2 * cp2.x + t3 * end.x,
        y: mt3 * start.y + 3.0 * mt2 * t * cp1.y + 3.0 * mt * t2 * cp2.y + t3 * end.y,
    }
}

pub fn distance_to_bezier(point: Position, start: Position, end: Position, steps: usize) -> f64 {
    let mut min_dist = f64::MAX;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let curve_point = bezier_point(start, end, t);
        let dx = point.x - curve_point.x;
        let dy = point.y - curve_point.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < min_dist {
            min_dist = dist;
        }
    }
    min_dist
}

pub fn snap_to_grid(pos: Position, grid_size: f64) -> Position {
    Position {
        x: (pos.x / grid_size).round() * grid_size,
        y: (pos.y / grid_size).round() * grid_size,
    }
}

pub fn distance(a: Position, b: Position) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}
