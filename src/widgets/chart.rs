use core::f64;
use std::marker::PhantomData;

use gdk::RGBA;
use glib::Continue;
use gtk::{prelude::WidgetExt, Inhibit};
use gtk::ResponseType::No;
use log::{error, info};

use crate::datahodler::ring::Ring;

#[derive(Clone)]
pub enum LineType {
    Line,
    Pillar,
}

#[derive(Clone)]
pub enum DrawDirection {
    TopDown,
    DownTop
}

#[derive(Clone)]
pub struct Series<E: Into<f64> + Clone> {
    id: String,
    max_value: E,
    ring: Ring<E>,
    color: RGBA,
    vari_height: bool,
}

impl<E: Into<f64> + Clone> Series<E> {
    pub fn new(
        id: &str,
        max_value: E,
        ring_size: usize,
        color: RGBA,
        vari_height: bool,
    ) -> Self {
        Series {
            id: id.to_string(),
            max_value,
            ring: Ring::new(ring_size),
            color,
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
    max_points: Option<usize>,
    line_type: Option<LineType>,
    phondata: PhantomData<E>,
}

impl<E: Into<f64> + Clone + 'static> Chart<E> {
    fn new(series: Vec<(Series<E>, DrawDirection)>, max_point: Option<usize>, line_type: LineType) -> Self {
        let drawing_area = gtk::DrawingArea::builder().vexpand(false).hexpand(true).build();


        drawing_area.connect_draw(glib::clone!(@strong series, @strong max_point => move |da, cr| {
            Self::draw_pillar(&series, max_point, da, cr);

            Inhibit(false)
        }));

        Self {
            drawing_area,
            max_points: None,
            line_type: None,
            phondata: Default::default()
        }
    }

    fn draw_pillar(series: &Vec<(Series<E>, DrawDirection)>, max_point: Option<usize>, da: &gtk::DrawingArea, cr: &gdk::cairo::Context) {
        let alloc = da.allocation();

        let width = alloc.width();
        let height = alloc.height();

        cr.scale(width as f64, height as f64);

        cr.set_source_rgba(0. / 255.0, 0. / 255.0, 0. / 255.0, 0.0);
        cr.paint().unwrap();

        let drawlen = max_point.as_ref().unwrap_or(&series.iter().map(|e| {e.0.ring.size}).max().unwrap_or(60)).clone();
        cr.set_line_width(1. / 2. / drawlen as f64);

        let slen = series.len();

        let onelen = drawlen / slen;
        let interval = 1. / drawlen as f64;

        for i in 0..slen {
            let serie = &series[i];
            cr.set_source_rgb(serie.0.color.red(), serie.0.color.green(),
                              serie.0.color.blue());
            let points: Vec<Point> = Self::scale(&serie.0, &serie.1).into_iter().take(onelen).collect();
            for (j, ele) in points.iter().enumerate() {
                let x = (j * slen + i) as f64 * interval;
                cr.move_to(x, match serie.1 {
                    DrawDirection::TopDown => {0.05}
                    DrawDirection::DownTop => {0.95}
                });
                cr.line_to(x, ele.y);
            }
            cr.stroke().unwrap();
        }
    }
	
    pub fn draw_in_seconds(&self, secs: u32) {
        let drawing_area = self.drawing_area.clone();
        glib::timeout_add_seconds_local(secs, move || {
            drawing_area.queue_draw();
            Continue(true)
        });
    }

    fn draw(series: &Vec<(Series<E>, DrawDirection)>, da: &gtk::DrawingArea, cr: &gdk::cairo::Context) {
        let alloc = da.allocation();

        let width = alloc.width();
        let height = alloc.height();

        cr.scale(width as f64, height as f64);

        cr.set_source_rgba(0. / 255.0, 0. / 255.0, 0. / 255.0, 0.0);
        cr.fill().unwrap();
        cr.translate(50., 50.);

        cr.set_line_width(1. / height as f64);

        for serie in series {
            let point_vec = Self::scale(&serie.0, &serie.1);
            cr.set_source_rgb(serie.0.color.red(), serie.0.color.green(), serie.0.color.blue());

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

    fn scale(series: &Series<E>, draw_direction: &DrawDirection) -> Vec<Point> {
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
                    y: match draw_direction {
                        DrawDirection::TopDown => {
                            0.1 + f64::min(1., sh) * 0.5
                        }
                        DrawDirection::DownTop => {
                            0.9 - f64::min(1., sh) * 0.5
                        }
                    },
                }
            })
            .collect()
    }

    pub fn builder() -> ChartBuilder<E> {
        ChartBuilder {
            height: None,
            width: None,
            series: vec![],
            line_type: None,
            max_points: None
        }
    }
}


pub struct ChartBuilder<E: Into<f64> + Clone> {
    height: Option<i32>,
    width: Option<i32>,
    series: Vec<(Series<E>, DrawDirection)>,
    line_type: Option<LineType>,
    max_points: Option<usize>
}

impl<E: Into<f64> + Clone + 'static> ChartBuilder<E> {
    pub fn height(mut self, height: i32) -> Self {
        self.height.replace(height);
        self
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width.replace(width);
        self
    }

    pub fn with_series(mut self, series: Series<E>, dir: DrawDirection) -> Self {
        self.series.push((series, dir));
        self
    }

    pub fn max_points(mut self, size: usize) -> Self {
        self.max_points.replace(size);

        self
    }

    pub fn line_type(mut self, line_type: LineType) -> Self {
        self.line_type.replace(line_type);

        self
    }

    pub fn build(self) -> Chart<E> {
        let line_type = match self.line_type {
            None => {
                LineType::Line
            }
            Some(t) => {
                t
            }
        };

        let mut chart = Chart::new(self.series, self.max_points, line_type);
        if let Some(h) = self.height {
            chart.drawing_area.set_height_request(h);
        }

        if let Some(w) = self.width {
            chart.drawing_area.set_width_request(w);
        }

        if let Some(mp) = self.max_points {
            chart.max_points.replace(mp);
        }



        chart
    }
}
