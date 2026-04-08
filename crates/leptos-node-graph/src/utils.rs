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
