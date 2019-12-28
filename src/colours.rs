#[derive(Copy, Clone, Debug)]
pub struct Colour {
    rgba: [f32; 4],
}

const EPSILON: f32 = 0.001;

impl PartialEq for Colour {
    fn eq(&self, other: &Self) -> bool {
        self.rgba
            .iter()
            .zip(other.rgba.iter())
            .all(|(a, b)| (a - b).abs() < EPSILON)
    }
}

impl Colour {
    fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            rgba: [red, green, blue, alpha],
        }
    }

    fn rgb_from_int_tuple(rgb: (i32, i32, i32)) -> Self {
        Self::new(
            rgb.0 as f32 / 255.0,
            rgb.1 as f32 / 255.0,
            rgb.2 as f32 / 255.0,
            1.0,
        )
    }

    fn red(&self) -> f32 {
        self.rgba[0]
    }

    fn green(&self) -> f32 {
        self.rgba[1]
    }

    fn blue(&self) -> f32 {
        self.rgba[2]
    }

    fn alpha(&self) -> f32 {
        self.rgba[3]
    }

    fn to_hsl(&self) -> [f32; 3] {
        let mut sorted_colour_values = [self.red(), self.green(), self.blue()];
        sorted_colour_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = sorted_colour_values.first().cloned().unwrap_or(0.0);
        let max = sorted_colour_values.last().cloned().unwrap_or(0.0);
        let luminance = (min + max) / 2.0;
        let saturation = if min == max {
            0.0
        } else {
            if luminance < 0.5 {
                (max - min) / (max + min)
            } else {
                (max - min) / (2.0 - max - min)
            }
        };
        let mut hue = if (self.red() == self.green()) && (self.green() == self.blue()) {
            0.0
        } else if self.red() == max {
            (self.green() - self.blue()) / (max - min)
        } else if self.green() == max {
            2.0 + (self.blue() - self.red()) / (max - min)
        } else if self.blue() == max {
            4.0 + (self.red() - self.green()) / (max - min)
        } else {
            0.0
        };
        hue *= 60.0;
        if hue < 0.0 {
            hue += 360.0;
        }
        hue /= 360.0;
        [hue, saturation, luminance]
    }

    fn from_hsl_tuple(hsl: (f32, f32, f32)) -> Self {
        let hue = hsl.0;
        let saturation = hsl.1;
        let luminance = hsl.2;
        if saturation == 0.0 {
            Colour::new(luminance, luminance, luminance, 1.0)
        } else {
            let c = (1.0 - (2.0 * luminance - 1.0).abs()) * saturation;
            let h = hue * 360.0;
            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
            let m = luminance - c / 2.0;
            let colours = if h < 60.0 {
                (c, x, 0.0)
            } else if h < 120.0 {
                (x, c, 0.0)
            } else if h < 180.0 {
                (0.0, c, x)
            } else if h < 240.0 {
                (0.0, x, c)
            } else if h < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };
            Colour::new(colours.0 + m, colours.1 + m, colours.2 + m, 1.0)
        }
    }

    fn from_hsl(hsl: [f32; 3]) -> Self {
        Self::from_hsl_tuple((hsl[0], hsl[1], hsl[2]))
    }

    pub fn lighten(&self, percentage: f32) -> Colour {
        let mut hsl = self.to_hsl();
        hsl[2] = f32::min(1.0, hsl[2] + percentage);
        Colour::from_hsl(hsl)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const TEST_CASES: [((i32, i32, i32), (f32, f32, f32)); 16] = [
        ((0, 0, 0), (0.0, 0.0, 0.0)),
        ((255, 255, 255), (0.0, 0.0, 1.0)),
        ((255, 0, 0), (0.0, 1.0, 0.5)),
        ((0, 255, 0), (120.0, 1.0, 0.5)),
        ((0, 0, 255), (240.0, 1.0, 0.5)),
        ((255, 255, 0), (60.0, 1.0, 0.5)),
        ((0, 255, 255), (180.0, 1.0, 0.5)),
        ((255, 0, 255), (300.0, 1.0, 0.5)),
        ((191, 191, 191), (0.0, 0.0, 0.75)),
        ((128, 128, 128), (0.0, 0.0, 0.5)),
        ((128, 0, 0), (0.0, 1.0, 0.25)),
        ((128, 128, 0), (60.0, 1.0, 0.25)),
        ((0, 128, 0), (120.0, 1.0, 0.25)),
        ((128, 0, 128), (300.0, 1.0, 0.25)),
        ((0, 128, 128), (180.0, 1.0, 0.25)),
        ((0, 0, 128), (240.0, 1.0, 0.25)),
    ];

    fn test_cases() -> impl Iterator<Item = (Colour, (f32, f32, f32))> {
        TEST_CASES.iter().map(|(rgb, hsl)| {
            (
                Colour::rgb_from_int_tuple(*rgb),
                (hsl.0 / 360.0, hsl.1, hsl.2),
            )
        })
    }

    #[test]
    fn test_rgb_to_hsl() {
        for test_case in test_cases() {
            let colour = test_case.0;
            let expected_hsl: Vec<f32> = [(test_case.1).0, (test_case.1).1, (test_case.1).2]
                .iter()
                .map(|component| (component * 100.0).round() / 100.0)
                .collect();
            let actual_hsl = colour.to_hsl();
            let rounded_hsl: Vec<f32> = actual_hsl
                .iter()
                .map(|component| (component * 100.0).round() / 100.0)
                .collect();
            assert_eq!(
                expected_hsl,
                &rounded_hsl[..],
                "{:?} should have converted to {:?}",
                test_case.0,
                test_case.1
            );
        }
    }

    #[test]
    fn test_from_hsl_tuple() {
        for test_case in test_cases() {
            let expected_color: Vec<f32> = (test_case.0)
                .rgba
                .iter()
                .map(|component| (component * 100.0).round() / 100.0)
                .collect();
            let color = Colour::from_hsl_tuple(test_case.1);
            let actual_color: Vec<f32> = color
                .rgba
                .iter()
                .map(|component| (component * 100.0).round() / 100.0)
                .collect();
            assert_eq!(
                expected_color, actual_color,
                "{:?} should have converted to {:?}",
                test_case.1, test_case.0
            );
        }
    }

    #[test]
    fn test_rgb_to_hsl_and_back_again() {
        let white = Colour::rgb_from_int_tuple((255, 255, 255));
        assert_eq!(white, Colour::from_hsl(white.to_hsl()));

        let black = Colour::rgb_from_int_tuple((0, 0, 0));
        assert_eq!(black, Colour::from_hsl(black.to_hsl()));

        let red = Colour::rgb_from_int_tuple((255, 0, 0));
        assert_eq!(red, Colour::from_hsl(red.to_hsl()));

        let green = Colour::rgb_from_int_tuple((0, 255, 0));
        assert_eq!(green, Colour::from_hsl(green.to_hsl()));

        let blue = Colour::rgb_from_int_tuple((0, 0, 255));
        assert_eq!(blue, Colour::from_hsl(blue.to_hsl()));
    }
}
