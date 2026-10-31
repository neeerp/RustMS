//! The Lie Detector's picture (report c025): a few letters and digits drawn
//! askew over noise, as the baseline JPEG the v83 client decodes, and the
//! text a tested player must type back.

use jpeg_encoder::{ColorType, Encoder};
use rand::Rng;

pub const WIDTH: usize = 220;
pub const HEIGHT: usize = 70;
const QUALITY: u8 = 80;

/// The characters it draws: no I, O, S, Z, B, Q or the digits like them.
const GLYPHS: [(char, [u8; 7]); 24] = [
    (
        'A',
        [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'C',
        [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
    ),
    (
        'D',
        [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
    ),
    (
        'E',
        [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
    ),
    (
        'F',
        [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
    ),
    (
        'G',
        [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
    ),
    (
        'H',
        [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'J',
        [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
    ),
    (
        'K',
        [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
    ),
    (
        'L',
        [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
    ),
    (
        'M',
        [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'N',
        [
            0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001,
        ],
    ),
    (
        'P',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
    ),
    (
        'R',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
    ),
    (
        'T',
        [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
    ),
    (
        'U',
        [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        'V',
        [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
    ),
    (
        'W',
        [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
    ),
    (
        'X',
        [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
    ),
    (
        'Y',
        [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
    ),
    (
        '3',
        [
            0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
        ],
    ),
    (
        '4',
        [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
    ),
    (
        '7',
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
    ),
    (
        '9',
        [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
    ),
];

pub struct Captcha {
    /// The text drawn, in capitals.
    pub answer: String,
    pub jpeg: Vec<u8>,
}

impl Captcha {
    /// Whether `typed` is the text drawn, whatever its case or the spaces
    /// around it.
    pub fn accepts(&self, typed: &str) -> bool {
        typed.trim().eq_ignore_ascii_case(&self.answer)
    }
}

struct Canvas {
    rgb: Vec<u8>,
}

impl Canvas {
    fn plot(&mut self, x: i32, y: i32, colour: [u8; 3]) {
        if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
            let at = (y as usize * WIDTH + x as usize) * 3;
            self.rgb[at..at + 3].copy_from_slice(&colour);
        }
    }
}

fn dark(rng: &mut impl Rng) -> [u8; 3] {
    [
        rng.gen_range(0, 110),
        rng.gen_range(0, 110),
        rng.gen_range(0, 110),
    ]
}

/// Four to six characters drawn as a JPEG.
pub fn generate(rng: &mut impl Rng) -> Captcha {
    let base = [
        rng.gen_range(200u8, 250),
        rng.gen_range(200u8, 250),
        rng.gen_range(200u8, 250),
    ];
    let mut canvas = Canvas {
        rgb: Vec::with_capacity(WIDTH * HEIGHT * 3),
    };
    for _ in 0..WIDTH * HEIGHT {
        for c in base {
            canvas.rgb.push(c.saturating_sub(rng.gen_range(0, 20)));
        }
    }

    let count = rng.gen_range(4, 7);
    let mut answer = String::new();
    let mut x = rng.gen_range(6, 20);
    for _ in 0..count {
        let (letter, rows) = GLYPHS[rng.gen_range(0, GLYPHS.len())];
        answer.push(letter);
        let scale = rng.gen_range(4, 6);
        let top = rng.gen_range(4, HEIGHT as i32 - 7 * scale - 4);
        let shear: f32 = rng.gen_range(-0.35, 0.35);
        let colour = dark(rng);
        for py in 0..7 * scale {
            let row = rows[(py / scale) as usize];
            let shift = (shear * (py as f32 - 3.5 * scale as f32)).round() as i32;
            for px in 0..5 * scale {
                if row & (0b10000 >> (px / scale)) != 0 {
                    canvas.plot(x + px + shift, top + py, colour);
                }
            }
        }
        x += 5 * scale + rng.gen_range(4, 9);
    }

    for _ in 0..rng.gen_range(4, 7) {
        let colour = dark(rng);
        let (x0, y0) = (
            rng.gen_range(0, WIDTH) as f32,
            rng.gen_range(0, HEIGHT) as f32,
        );
        let (x1, y1) = (
            rng.gen_range(0, WIDTH) as f32,
            rng.gen_range(0, HEIGHT) as f32,
        );
        let steps = (x1 - x0).abs().max((y1 - y0).abs()) as i32 + 1;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let px = (x0 + (x1 - x0) * t).round() as i32;
            let py = (y0 + (y1 - y0) * t).round() as i32;
            canvas.plot(px, py, colour);
        }
    }
    for _ in 0..WIDTH * HEIGHT / 25 {
        let colour = dark(rng);
        canvas.plot(
            rng.gen_range(0, WIDTH as i32),
            rng.gen_range(0, HEIGHT as i32),
            colour,
        );
    }

    let mut jpeg = Vec::new();
    Encoder::new(&mut jpeg, QUALITY)
        .encode(&canvas.rgb, WIDTH as u16, HEIGHT as u16, ColorType::Rgb)
        .expect("a JPEG of a fixed size encodes");
    Captcha { answer, jpeg }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn draws_four_to_six_known_characters_as_a_baseline_jpeg() {
        let mut rng = StdRng::seed_from_u64(83);
        for _ in 0..50 {
            let captcha = generate(&mut rng);
            assert!((4..=6).contains(&captcha.answer.len()));
            assert!(captcha
                .answer
                .chars()
                .all(|c| GLYPHS.iter().any(|(g, _)| *g == c)));
            assert_eq!(captcha.jpeg[..2], [0xFF, 0xD8]);
            assert_eq!(captcha.jpeg[captcha.jpeg.len() - 2..], [0xFF, 0xD9]);
            // A baseline frame (SOF0), never a progressive one (SOF2).
            let sof0 = captcha.jpeg.windows(2).any(|w| w == [0xFF, 0xC0]);
            let sof2 = captcha.jpeg.windows(2).any(|w| w == [0xFF, 0xC2]);
            assert!(sof0 && !sof2);
        }
    }

    #[test]
    fn takes_the_answer_in_any_case() {
        let captcha = Captcha {
            answer: "AK47".into(),
            jpeg: Vec::new(),
        };
        assert!(captcha.accepts("ak47"));
        assert!(captcha.accepts(" Ak47 "));
        assert!(!captcha.accepts("ak4"));
        assert!(!captcha.accepts(""));
    }
}
