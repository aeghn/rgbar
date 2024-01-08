use core::f64;
use std::{cell::RefCell, cmp, marker::PhantomData};

use gdk::RGBA;
use glib::{Continue, MainContext};
use gtk::{prelude::WidgetExt, subclass::drawing_area, true_, Inhibit};

use crate::datahodler::ring::Ring;

#[derive(Clone)]
pub enum SeriesType {
    Line,
    Bar,
    Pillar,
}

#[derive(Clone)]
pub struct Series<E: Into<f64> + Clone> {
    id: String,
    max_value: E,
    ring: Ring<E>,
    color: RGBA,
    draw_type: SeriesType,
    vari_height: bool,
}

impl<E: Into<f64> + Clone> Series<E> {
    pub fn new(
        id: &str,
        max_value: E,
        ring_size: usize,
        color: RGBA,
        draw_type: SeriesType,
        vari_height: bool,
    ) -> Self {
        Series {
            id: id.to_string(),
            max_value: max_value,
            ring: Ring::new(ring_size),
            color,
            draw_type,
            vari_height,
        }
    }

    pub fn add_value(&self, value: E) {
        self.ring.add(value);
    }
}

struct Point {
    x: f64,
    y: f64,
}

pub struct Chart<E: Into<f64> + Clone> {
    pub drawing_area: gtk::DrawingArea,
    phondata: PhantomData<E>,
}

impl<E: Into<f64> + Clone + 'static> Chart<E> {
    pub fn new(series: Vec<Series<E>>, height: i32) -> Self {
        let drawing_area = gtk::DrawingArea::builder()
            .height_request(height)
            .width_request(100)
            .build();

        drawing_area.connect_draw(glib::clone!(@strong series => move |da, cr| {
            Self::draw(&series, da, cr);

            Inhibit(false)
        }));

        {
            let drawing_area = drawing_area.clone();
            glib::timeout_add_seconds_local(1, move || {
                drawing_area.queue_draw();
                Continue(true)
            });
        }

        Self {
            drawing_area,
            phondata: Default::default(),
        }
    }

    fn draw(series: &Vec<Series<E>>, da: &gtk::DrawingArea, cr: &gdk::cairo::Context) {
        let alloc = da.allocation();

        let width = alloc.width();
        let height = alloc.height();

        cr.scale(width as f64, height as f64);

        cr.set_source_rgba(0. / 255.0, 0. / 255.0, 0. / 255.0, 0.0);
        cr.paint().unwrap();

        cr.set_line_width(0.01);

        for serie in series {
            let point_vec = Self::scale(serie);
            cr.set_source_rgb(serie.color.red(), serie.color.green(), serie.color.blue());

            if point_vec.len() <= 1 {
                continue;
            }

            cr.move_to(point_vec[0].x.into(), point_vec[1].y.into());

            for ele in point_vec.iter().skip(1) {
                cr.line_to(ele.x as f64, ele.y as f64);
            }

            cr.stroke().unwrap();
        }
    }

    fn scale(series: &Series<E>) -> Vec<Point> {
        let maw: f64 = series.ring.size as f64;

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
            .enumerate()
            .map(|(id, h)| {
                let sh = h / mah;
                Point {
                    x: id as f64 / maw,
                    y: 1. - (if sh > 1 as f64 { 1f64 } else { sh }),
                }
            })
            .collect()
    }

    pub fn builder() -> ChartBuilder<E> {
        ChartBuilder {
            heiget: 20,
            width: 80,
            series: vec![],
        }
    }
}

pub struct ChartBuilder<E: Into<f64> + Clone> {
    heiget: i32,
    width: i32,
    series: Vec<Series<E>>,
}

impl<E: Into<f64> + Clone + 'static> ChartBuilder<E> {
    pub fn height(mut self, height: i32) -> Self {
        self.heiget = height;
        self
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = width;
        self
    }

    pub fn with_series(mut self, series: Series<E>) -> Self {
        self.series.push(series);
        self
    }

    pub fn build(self) -> Chart<E> {
        let chart = Chart::new(self.series, self.heiget);
        chart.drawing_area.set_width_request(self.width);

        chart
    }
}
