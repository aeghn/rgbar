use core::f64;
use std::marker::PhantomData;

use gdk::glib::Propagation;
use gdk::RGBA;
use gtk::prelude::BoxExt;

use gtk::prelude::WidgetExt;

use crate::datahodler::ring::Ring;

#[derive(Clone)]
pub enum LineType {
    Line,
    Pillar,
}

#[derive(Clone)]
pub struct Series<E: Into<f64> + Clone> {
    id: String,
    max_value: E,
    ring: Ring<E>,
    color: RGBA,
    vari_height: bool,
    baseline_percent: f64,
    height_percent: f64,
}

impl<E: Into<f64> + Clone> Series<E> {
    pub fn new(id: &str, max_value: E, ring_size: usize, color: RGBA, vari_height: bool) -> Self {
        Series {
            id: id.to_string(),
            max_value,
            ring: Ring::new(ring_size),
            color,
            vari_height,
            baseline_percent: 0.0,
            height_percent: 1.0,
        }
    }

    pub fn add_value(&self, value: E) {
        self.ring.add(value);
    }

    pub fn set_baseline_and_height(&mut self, base: f64, height: f64) {
        self.baseline_percent = base;
        self.height_percent = height;
    }
}

struct Point {
    x: f64,
    y: f64,
}

pub struct Chart<E: Into<f64> + Clone> {
    drawing_area: gtk::DrawingArea,
    line_width: f64,
    line_type: LineType,
    phondata: PhantomData<E>,
    pub drawing_box: gtk::Box,
    series: Vec<Series<E>>,
}

impl<E: Into<f64> + Clone + 'static> Chart<E> {
    fn new() -> Self {
        let drawing_area = gtk::DrawingArea::builder()
            .vexpand(false)
            .hexpand(true)
            .build();

        let drawing_box = gtk::Box::builder().build();

        drawing_box.pack_start(&drawing_area, true, true, 0);

        Self {
            drawing_area,
            line_width: 1.0,
            line_type: LineType::Line,
            phondata: Default::default(),
            drawing_box,
            series: vec![],
        }
    }

    pub fn draw_in_seconds(&self, secs: u32) {
        let series = self.series.clone();
        let line_width = self.line_width.clone();
        self.drawing_area.connect_draw(
            glib::clone!(@strong series, @strong line_width => move |da, cr| {
                Self::draw(&series, line_width, da, cr);
                Propagation::Proceed
            }),
        );

        let drawing_area = self.drawing_area.clone();
        glib::timeout_add_seconds_local(secs, move || {
            drawing_area.queue_draw();

            glib::ControlFlow::Continue
        });
    }

    fn draw(
        series: &Vec<Series<E>>,
        line_width: f64,
        da: &gtk::DrawingArea,
        cr: &gdk::cairo::Context,
    ) {
        let alloc = da.allocation();

        let width = alloc.width() as f64;
        let height = alloc.height() as f64;

        //cr.set_source_rgba(0.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0, 0.0);
        // cr.paint().unwrap();

        cr.set_line_width(line_width);
        let max_ring_size = series.iter().map(|s| s.ring.size).max().unwrap_or(30);
        let interval = 1.0 / ((max_ring_size - 2) as f64);
        cr.translate(-1. * width * interval, 0.);

        for serie in series {
            let point_height: Vec<f64> = Self::scale(&serie).into_iter().collect();

            if point_height.len() <= 1 {
                continue;
            }

            let transform_y =
                |v: f64| (1. - (v * serie.height_percent + serie.baseline_percent)) * height;

            let transform_x = |v| (1.0 - v as f64 * interval) * width;

            let start = (transform_x(0), transform_y(point_height[0]));

            cr.move_to(start.0, start.1);
            cr.set_source_rgb(serie.color.red(), serie.color.green(), serie.color.blue());

            let mut end = start.clone();
            for (i, ele) in point_height.iter().skip(1).enumerate() {
                end = (transform_x(i + 1), transform_y(ele.clone()));
                cr.line_to(end.0.clone(), end.1.clone());
            }
            cr.stroke_preserve().unwrap();

            cr.move_to(start.0, start.1);
            for (i, ele) in point_height.iter().skip(1).enumerate() {
                end = (transform_x(i + 1), transform_y(ele.clone()));
                cr.line_to(end.0.clone(), end.1.clone());
            }

            cr.line_to(end.0, transform_y(0.));
            cr.line_to(start.0, transform_y(0.));
            cr.line_to(start.0, start.1);

            cr.set_source_rgba(
                serie.color.red(),
                serie.color.green(),
                serie.color.blue(),
                serie.color.alpha(),
            );
            cr.fill().unwrap();
        }
    }

    fn scale(series: &Series<E>) -> Vec<f64> {
        let all: Vec<f64> = series
            .ring
            .get_all()
            .into_iter()
            .map(|e| e.into())
            .collect();

        let max_def: f64 = series.max_value.clone().into();
        let max = all
            .iter()
            .max_by(|e1, e2| e1.total_cmp(e2))
            .unwrap_or(&max_def)
            .clone();

        let mah: f64 = f64::max(max, max_def);

        all.into_iter()
            .rev()
            .map(|h| {
                let sh = h / mah;
                f64::min(1.0, sh)
            })
            .collect()
    }

    /*     pub fn with_height(self, height: i32) -> Self {
        self.drawing_area.set_height_request(height);
        self
    } */

    pub fn with_width(self, width: i32) -> Self {
        self.drawing_area.set_width_request(width);

        self
    }

    pub fn with_line_width(mut self, line_width: f64) -> Self {
        self.line_width = line_width;

        self
    }

    pub fn with_line_type(mut self, line_type: LineType) -> Self {
        self.line_type = line_type;

        self
    }

    pub fn with_series(mut self, series: Series<E>) -> Self {
        if !self.series.is_empty() {
            if self.series[0].ring.size != series.ring.size {
                tracing::warn!("the series should have same sizes.");
            }
        }

        self.series.push(series);
        self
    }

    pub(crate) fn builder() -> Self {
        Self::new()
    }
}
