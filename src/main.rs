use std::{
    path::PathBuf,
    time::{
        Instant,
        Duration,
    },
};

use rand::Rng;

use piston_window::{
    OpenGL,
    PistonWindow,
    WindowSettings,
    Viewport,
    PressEvent,
    EventLoop,
    Button,
    Key
};

use rustdct::{
    DCTplanner,
};

const CONSOLE_HEIGHT: u32 = 32;
const BORDER_WIDTH: u32 = 16;
const SCREEN_WIDTH: u32 = 640;
const SCREEN_HEIGHT: u32 = 480;

fn main() {
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("periodogram", [SCREEN_WIDTH, SCREEN_HEIGHT])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    let mut font_path = PathBuf::from("assets");
    font_path.push("FiraSans-Regular.ttf");
    let mut glyphs = window.load_font(font_path).unwrap();
    window.set_lazy(true);

    let amplitude = (-3.3, 3.3);
    let points = noised_signal_gen(amplitude.1, 50.0, 0.1, Duration::from_micros(50_000), 128, false);

    let mut dct = Dct::new(DctParams {
        outputs_count: 128,
        coeffs_count: 16,
    });

    dct.apply(&points, amplitude);

    while let Some(event) = window.next() {
        window.draw_2d(&event, |context, g2d, device| {
            use piston_window::{clear, text, line, Transformed};
            clear([0.0, 0.0, 0.0, 1.0], g2d);
            let trans = context.transform.trans(5.0, 20.0);
            text::Text::new_color([0.0, 1.0, 0.0, 1.0], 16).draw(
                &format!("#outputs: A/S [ {} ], #coeffs Z/X [ {} ]", dct.params.outputs_count, dct.params.coeffs_count),
                &mut glyphs,
                &context.draw_state,
                trans,
                g2d,
            ).unwrap();
            glyphs.factory.encoder.flush(device);

            if let Some(tr) = ViewportTranslator::new(&context.viewport) {
                draw(&points, amplitude, dct.outputs(), |element| match element {
                    DrawElement::Line { color, radius, source_x, source_y, target_x, target_y } => {
                        line(color, radius, [tr.x(source_x), tr.y(source_y), tr.x(target_x), tr.y(target_y)], context.transform, g2d);
                        line(
                            color,
                            radius,
                            [tr.x(source_x - 0.005), tr.y(source_y - 0.005), tr.x(source_x + 0.005), tr.y(source_y + 0.005)],
                            context.transform,
                            g2d,
                        );
                        line(
                            color,
                            radius,
                            [tr.x(source_x - 0.005), tr.y(source_y + 0.005), tr.x(source_x + 0.005), tr.y(source_y - 0.005)],
                            context.transform,
                            g2d,
                        );
                    },
                });
            }
        });

        match event.press_args() {
            Some(Button::Keyboard(Key::A)) if dct.params.outputs_count > 1 => {
                dct.params.outputs_count -= 1;
                dct.apply(&points, amplitude);
            },
            Some(Button::Keyboard(Key::S)) if dct.params.outputs_count < points.len() => {
                dct.params.outputs_count += 1;
                dct.apply(&points, amplitude);
            },
            Some(Button::Keyboard(Key::Z)) if dct.params.coeffs_count > 1 => {
                dct.params.coeffs_count -= 1;
                dct.apply(&points, amplitude);
            },
            Some(Button::Keyboard(Key::X)) if dct.params.coeffs_count < 16 => {
                dct.params.coeffs_count += 1;
                dct.apply(&points, amplitude);
            },
            Some(Button::Keyboard(Key::Q)) =>
                break,
            _ =>
                (),
        }
    }
}

enum DrawElement {
    Line {
        color: [f32; 4],
        radius: f64,
        source_x: f64,
        source_y: f64,
        target_x: f64,
        target_y: f64,
    },
}

struct ViewportTranslator {
    scale_x: f64,
    scale_y: f64,
    min_x: f64,
    min_y: f64,
}

impl ViewportTranslator {
    fn x(&self, x: f64) -> f64 {
        (x - self.min_x) * self.scale_x + BORDER_WIDTH as f64
    }

    fn y(&self, y: f64) -> f64 {
        (y - self.min_y) * self.scale_y + CONSOLE_HEIGHT as f64
    }

    fn new(viewport: &Option<Viewport>) -> Option<ViewportTranslator> {
        let (w, h) = viewport
            .map(|v| (v.draw_size[0], v.draw_size[1]))
            .unwrap_or((SCREEN_WIDTH, SCREEN_HEIGHT));

        if (w <= 2 * BORDER_WIDTH) || (h <= BORDER_WIDTH + CONSOLE_HEIGHT) {
            None
        } else {
            let ((l, r), (t, b)) = ((0., 1.0), (0., 1.0));

            Some(ViewportTranslator {
                scale_x: (w - BORDER_WIDTH - BORDER_WIDTH) as f64 / (r - l),
                scale_y: (h - BORDER_WIDTH - CONSOLE_HEIGHT) as f64 / (b - t),
                min_x: l,
                min_y: t,
            })
        }
    }
}

fn draw<DF>(points: &[Reading], amplitude: (f64, f64), dct_output: &[f64], mut draw_element: DF) where DF: FnMut(DrawElement) {
    if let Some(&Reading { when: max_when, .. }) = points.last() {
        let mut plots_iter = points.iter();
        let first_reading = plots_iter.next().unwrap();

        let mut dct_iter = Some(dct_output.iter());
        let mut prev_dct_value = dct_iter
            .as_mut()
            .and_then(|iter| iter.next())
            .unwrap();

        let mut prev_reading = first_reading;
        while let Some(reading) = plots_iter.next() {
            let source_x = 1.0 - ((max_when - prev_reading.when).as_secs_f64() / (max_when - first_reading.when).as_secs_f64());
            let target_x = 1.0 - ((max_when - reading.when).as_secs_f64() / (max_when - first_reading.when).as_secs_f64());
            draw_element(DrawElement::Line {
                color: [0., 0., 1., 1.,],
                radius: 0.5,
                source_x,
                source_y: (prev_reading.value - amplitude.0) / (amplitude.1 - amplitude.0),
                target_x,
                target_y: (reading.value - amplitude.0) / (amplitude.1 - amplitude.0),
            });
            if let Some(dct_value) = dct_iter.as_mut().and_then(|iter| iter.next()) {
                draw_element(DrawElement::Line {
                    color: [1., 1., 0., 1.,],
                    radius: 1.0,
                    source_x,
                    source_y: (prev_dct_value - amplitude.0) / (amplitude.1 - amplitude.0),
                    target_x,
                    target_y: (dct_value - amplitude.0) / (amplitude.1 - amplitude.0),
                });
                prev_dct_value = dct_value;
            }
            prev_reading = reading;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Reading {
    value: f64,
    when: Instant,
}

fn noised_signal_gen(
    amplitude: f64,
    freq: f64,
    noise_fraq: f64,
    duration: Duration,
    samples: usize,
    force_abs: bool,
)
    -> Vec<Reading>
{
    let mut rng = rand::thread_rng();
    let duration_f64 = duration.as_secs_f64();
    let now = Instant::now();
    let noise_amplitude = amplitude * noise_fraq;
    let mut readings: Vec<_> = (0 .. samples)
        .map(|_| {
            let time = rng.gen_range(0.0, duration_f64);
            let wave_arg = 2.0 * std::f64::consts::PI * freq * time;
            let wave_fun = amplitude * wave_arg.sin();
            let noise = rng.gen_range(-noise_amplitude, noise_amplitude);
            let mut value = wave_fun + noise;
            if force_abs {
                value = value.abs();

                if value > amplitude {
                    value = amplitude;
                }
            } else {
                if value < -amplitude {
                    value = -amplitude;
                } else if value > amplitude {
                    value = amplitude;
                }
            }
            Reading {
                value,
                when: now + Duration::from_secs_f64(time),
            }
        })
        .collect();
    readings.sort_by(|a, b| a.when.cmp(&b.when));
    readings
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct DctParams {
    coeffs_count: usize,
    outputs_count: usize,
}

struct Dct {
    planner: DCTplanner<f64>,
    dct_input: Vec<f64>,
    dct_coeffs: Vec<f64>,
    dct_output: Vec<f64>,
    params: DctParams,
}

impl Dct {
    fn new(params: DctParams) -> Dct {
        Dct {
            planner: DCTplanner::new(),
            dct_input: Vec::new(),
            dct_coeffs: Vec::new(),
            dct_output: Vec::new(),
            params,
        }
    }

    fn outputs(&self) -> &[f64] {
        &self.dct_output
    }

    fn apply(&mut self, points: &[Reading], amplitude: (f64, f64)) {
        self.dct_input.clear();
        self.dct_input.extend(
            points[.. self.params.outputs_count]
                .iter()
                .map(|reading| {
                    (reading.value - amplitude.0) / (amplitude.1 - amplitude.0)
                })
        );

        // prepare coeffs
        self.dct_coeffs.resize(self.params.outputs_count, 0.0);
        self.planner
            .plan_dct2(self.params.outputs_count)
            .process_dct2(&mut self.dct_input, &mut self.dct_coeffs);

        // leave only some coeffs
        for coeff in self.dct_coeffs.iter_mut().skip(self.params.coeffs_count) {
            *coeff = 0.0;
        }
        self.dct_coeffs.resize(self.params.outputs_count, 0.0);

        // restore wave
        self.dct_output.resize(self.params.outputs_count, 0.0);
        self.planner
            .plan_dct3(self.params.outputs_count)
            .process_dct3(&mut self.dct_coeffs, &mut self.dct_output);

        for v in self.dct_output.iter_mut() {
            *v *= 2.0 / self.params.outputs_count as f64;
            *v *= amplitude.1 - amplitude.0;
            *v += amplitude.0;
        }
    }
}
