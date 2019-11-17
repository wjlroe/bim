use glam::{vec2, Vec2};

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub top_left: Vec2,
    pub center: Vec2,
    pub bounds: Vec2,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            top_left: vec2(0.0, 0.0),
            center: vec2(0.0, 0.0),
            bounds: vec2(0.0, 0.0),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum CoordsSet {
    None,
    Center,
    TopLeft,
}

impl Default for CoordsSet {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Copy, Clone, Default)]
pub struct RectBuilder {
    rect: Rect,
    coords_set: CoordsSet,
}

impl RectBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bounds(mut self, bounds: Vec2) -> Self {
        self.rect.bounds = bounds;
        self
    }

    pub fn top_left(mut self, top_left: Vec2) -> Self {
        self.rect.top_left = top_left;
        self.coords_set = CoordsSet::TopLeft;
        self
    }

    pub fn center(mut self, center: Vec2) -> Self {
        self.rect.center = center;
        self.coords_set = CoordsSet::Center;
        self
    }

    fn calc_top_left(&mut self) {
        self.rect.top_left = vec2(
            self.rect.center.x() - self.rect.bounds.x() / 2.0,
            self.rect.center.y() - self.rect.bounds.y() / 2.0,
        );
    }

    fn calc_center(&mut self) {
        self.rect.center = vec2(
            self.rect.top_left.x() + self.rect.bounds.x() / 2.0,
            self.rect.top_left.y() + self.rect.bounds.y() / 2.0,
        );
    }

    pub fn build(mut self) -> Rect {
        match self.coords_set {
            CoordsSet::Center => self.calc_top_left(),
            CoordsSet::TopLeft => self.calc_center(),
            CoordsSet::None => {}
        };
        self.rect
    }
}

#[test]
fn test_no_position_rect() {
    let builder = RectBuilder::new().bounds(vec2(10.0, 10.0));
    assert_eq!(CoordsSet::None, builder.coords_set);
}

#[test]
fn test_center_from_top_left() {
    let builder = RectBuilder::new()
        .top_left(vec2(10.0, 5.0))
        .bounds(vec2(24.0, 4.0));
    assert_eq!(CoordsSet::TopLeft, builder.coords_set);
    let rect = builder.build();
    assert_eq!(vec2(22.0, 7.0), rect.center);
}

#[test]
fn top_left_from_center() {
    let builder = RectBuilder::new()
        .bounds(vec2(24.0, 4.0))
        .center(vec2(22.0, 7.0));
    assert_eq!(CoordsSet::Center, builder.coords_set);
    let rect = builder.build();
    assert_eq!(vec2(10.0, 5.0), rect.top_left);
}
