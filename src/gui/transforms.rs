use crate::rect::Rect;
use glam::{vec3, Mat4, Vec2};

pub struct Transforms {
    pub window_dim: Vec2,
}

impl Transforms {
    pub fn new(window_dim: Vec2) -> Self {
        Self { window_dim }
    }

    pub fn transform_for_quad(&self, rect: Rect) -> Mat4 {
        let quad_scale = Mat4::from_scale(vec3(
            rect.bounds.x() / self.window_dim.x(),
            rect.bounds.y() / self.window_dim.y(),
            1.0,
        ));
        let position = rect.center;
        let x_translate = (position.x() / self.window_dim.x()) * 2.0 - 1.0;
        let y_translate = -((position.y() / self.window_dim.y()) * 2.0 - 1.0);
        let quad_translate = Mat4::from_translation(vec3(x_translate, y_translate, 0.0));
        quad_translate * quad_scale
    }
}

#[test]
fn test_quad_filling_bounds_should_be_identity_matrix() {
    use crate::rect::RectBuilder;
    use glam::{vec2, vec4};

    let transforms = Transforms::new(vec2(10.0, 10.0));
    let rect = RectBuilder::new()
        .top_left(vec2(0.0, 0.0))
        .bounds(vec2(10.0, 10.0))
        .build();
    let matrix = transforms.transform_for_quad(rect);
    let center_point = vec4(0.0, 0.0, 0.0, 0.0);
    assert_eq!(
        center_point,
        matrix * center_point,
        "center point shouldn't be affected"
    );
    let top_left_point = vec4(-1.0, -1.0, 0.0, 0.0);
    assert_eq!(
        top_left_point,
        matrix * top_left_point,
        "top_left point shouldn't be affected"
    );
    let bottom_right_point = vec4(1.0, 1.0, 0.0, 0.0);
    assert_eq!(
        bottom_right_point,
        matrix * bottom_right_point,
        "bottom_right point shouldn't be affected"
    );
}
