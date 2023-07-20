use colorous::MAGMA;
use gtk::{
    glib, glib::clone, prelude::*, Application, ApplicationWindow, DrawingArea, EventControllerKey,
    GestureClick,
};
use rayon::prelude::*;
use std::cell::Cell;
use std::f64::consts::E;
use std::rc::Rc;

const APP_ID: &str = "org.gtk_rs.mandelbrot";

const MAX_ITER: u32 = 200;
const DIVERGE_TRESH_SQ: f64 = 4.0;
const CANVAS_W: u32 = 1280;
const CANVAS_H: u32 = 720;
const ZOOM_INC: f64 = 2.0;

// Precision used for computations
use f64 as unit;

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn cmap(i: f64) -> [u8; 3] {
    let color = MAGMA.eval_continuous(i);
    [color.r, color.g, color.b]
}

fn step(x: &mut unit, y: &mut unit, x2: &mut unit, y2: &mut unit, x0: unit, y0: unit) {
    *y = (*x + *x) * *y + y0;
    *x = *x2 - *y2 + x0;
    *x2 = *x * *x;
    *y2 = *y * *y;
}

fn does_converge(c: [unit; 2]) -> f64 {
    let mut x: unit = 0.0;
    let mut y: unit = 0.0;
    let mut x2: unit = 0.0;
    let mut y2: unit = 0.0;
    for i in 1..=MAX_ITER {
        step(&mut x, &mut y, &mut x2, &mut y2, c[0], c[1]);
        if x2 + y2 > DIVERGE_TRESH_SQ as unit {
            return i as unit + 1 as unit - (x2 + y2).sqrt().log(E as unit).log(2.0) as unit;
        }
    }
    (MAX_ITER + 1) as unit
}

fn compute_line(line: Vec<[unit; 2]>, l: unit, r: unit, b: unit, t: unit) -> Vec<f64> {
    let result: Vec<f64> = line
        .iter()
        .map(|&e| {
            to_sub_values(e, l, r, b, t)
                .iter()
                .map(|&v| -> unit { does_converge(v) })
                .sum::<unit>() as f64
                / 4f64
        })
        .collect();
    result
}

fn to_sub_values(v: [unit; 2], l: unit, r: unit, b: unit, t: unit) -> [[unit; 2]; 4] {
    let pixel_width = (r - l) / CANVAS_W as unit;
    let pixel_height = (t - b) / CANVAS_H as unit;
    let x_inc = pixel_width / 2.0;
    let y_inc = pixel_height / 2.0;

    [
        [
            v[0] + 0.0 * x_inc + x_inc / 2.0,
            v[1] + 0.0 * y_inc + y_inc / 2.0,
        ],
        [
            v[0] + 0.0 * x_inc + x_inc / 2.0,
            v[1] + 1.0 * y_inc + y_inc / 2.0,
        ],
        [
            v[0] + 1.0 * x_inc + x_inc / 2.0,
            v[1] + 0.0 * y_inc + y_inc / 2.0,
        ],
        [
            v[0] + 1.0 * x_inc + x_inc / 2.0,
            v[1] + 1.0 * y_inc + y_inc / 2.0,
        ],
    ]
}

fn map_into_bounds(re: unit, im: unit, l: unit, r: unit, b: unit, t: unit) -> [unit; 2] {
    let x_span = r - l;
    let y_span = t - b;
    [
        re / CANVAS_W as unit * x_span + l,
        im / CANVAS_H as unit * y_span + b,
    ]
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Mandelbrot Visualizer")
        .build();

    let canvas = DrawingArea::new();
    canvas.set_size_request(1280, 720);
    let l = Rc::new(Cell::new(-2.96444));
    let r = Rc::new(Cell::new(1.44444));
    let b = Rc::new(Cell::new(-1.24));
    let t = Rc::new(Cell::new(1.24));

    canvas.set_draw_func(
        clone!(@strong l, @strong r, @strong b, @strong t => move |_, cr, _, _| {
            let int_grid: Vec<Vec<[u32; 2]>> = (0..CANVAS_H)
                .map(move |line| {
                    (0..CANVAS_W)
                        .map(move |e| [e as u32, line as u32])
                        .collect()
                })
                .collect();
            let _l = l.get();
            let _r = r.get();
            let _b = b.get();
            let _t = t.get();
            let int_result: Vec<Vec<f64>> = int_grid
                .into_par_iter()
                .map(|line| {
                    compute_line(
                        line.iter()
                            .map(|e| map_into_bounds(e[0] as unit, e[1] as unit, _l, _r, _b, _t))
                            .collect(),
                        _l,
                        _r,
                        _b,
                        _t,
                    )
                })
                .collect();
            int_result.iter().enumerate().for_each(|(i, line)| {
                line.iter().enumerate().for_each(|(j, elem)| {
                    cr.rectangle(j as f64, i as f64, 1.0, 1.0);
                    let c = match *elem < MAX_ITER as f64 {
                        true => cmap(*elem / MAX_ITER as f64),
                        false => cmap(0.0),
                    };
                    cr.set_source_rgb(
                        c[0] as f64 / 256.0 as f64,
                        c[1] as f64 / 256.0,
                        c[2] as f64 / 256.0,
                    );
                    cr.fill().unwrap();
                })
            })
        }),
    );
    let gesture = GestureClick::new();
    gesture.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
    gesture.connect_pressed(
        clone!(@strong l, @strong r, @strong b, @strong t, @weak canvas => move |gesture, _, x, y| {
            let [_x, _y] = map_into_bounds(x, y, l.get(), r.get(), b.get(), t.get());
            let h = t.get() - b.get();
            let w = r.get() - l.get();
            let cx = l.get() + w / 2.0;
            let cy = b.get() + h / 2.0;
            l.set(l.get() - cx + _x);
            r.set(r.get() - cx + _x);
            b.set(b.get() - cy + _y);
            t.set(t.get() - cy + _y);
            gesture.set_state(gtk::EventSequenceState::Claimed);
            canvas.queue_draw();
        }),
    );
    let key_controller = EventControllerKey::new();
    key_controller.connect_key_released(clone!(@strong l, @strong r, @strong b, @strong t, @weak canvas => move |_eventctl, _keyval, keycode, _state| {
        match keycode {
            86 => {
                let h = t.get() - b.get();
                let w = r.get() - l.get();
                let cx = l.get() + w / 2.0;
                let cy = b.get() + h / 2.0;
                l.set(cx - w/ZOOM_INC/2.0);
                r.set(cx + w/ZOOM_INC/2.0);
                b.set(cy - h/ZOOM_INC/2.0);
                t.set(cy + h/ZOOM_INC/2.0);
                canvas.queue_draw();
            },
            82 => {
                let h = t.get() - b.get();
                let w = r.get() - l.get();
                let cx = l.get() + w / 2.0;
                let cy = b.get() + h / 2.0;
                l.set(cx - w*ZOOM_INC/2.0);
                r.set(cx + w*ZOOM_INC/2.0);
                b.set(cy - h*ZOOM_INC/2.0);
                t.set(cy + h*ZOOM_INC/2.0);
                canvas.queue_draw();

            },
            _ => {}
        };
    }));
    window.add_controller(key_controller);
    canvas.add_controller(gesture);
    window.set_child(Some(&canvas));
    window.present();
}
