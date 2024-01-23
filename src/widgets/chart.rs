use core::f64;
use std::marker::PhantomData;

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
pub enum DrawDirection {
    TopDown,
    DownTop,
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
    pub fn new(id: &str, max_value: E, ring_size: usize, color: RGBA, vari_height: bool) -> Self {
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
    drawing_area: gtk::DrawingArea,
    line_width: Option<usize>,
    line_type: Option<LineType>,
    phondata: PhantomData<E>,
    pub drawing_box: gtk::Box,
}

impl<E: Into<f64> + Clone + 'static> Chart<E> {
    fn new(
        series: Vec<(Series<E>, DrawDirection)>,
        line_width: Option<usize>,
        line_type: LineType,
    ) -> Self {
        let drawing_area = gtk::DrawingArea::builder()
            .vexpand(false)
            .hexpand(true)
            .build();

        let drawing_box = gtk::Box::builder().build();

        drawing_area.connect_draw(
            glib::clone!(@strong series, @strong line_width => move |da, cr| {
                match line_type {
                    LineType::Line => {
                        Self::draw(&series, line_width, da, cr);
                    },
                    LineType::Pillar => {
                        Self::draw_up_and_down(&series, line_width, da, cr);
                    },
                }


                glib::Propagation::Proceed
            }),
        );

        drawing_box.pack_start(&drawing_area, true, true, 0);

        Self {
            drawing_area,
            line_width: None,
            line_type: None,
            phondata: Default::default(),
            drawing_box,
        }
    }

    fn draw_pillar(
        series: &Vec<(Series<E>, DrawDirection)>,
        line_width: Option<usize>,
        da: &gtk::DrawingArea,
        cr: &gdk::cairo::Context,
    ) {
        let alloc = da.allocation();

        let width = alloc.width();
        let height = alloc.height();

        cr.scale(width as f64, height as f64);

        cr.set_source_rgba(0. / 255.0, 0. / 255.0, 0. / 255.0, 0.0);
        cr.paint().unwrap();

        let line_width = line_width.unwrap_or(3);
        let interval = line_width as f64 / width as f64;
        let drawlen = width / line_width as i32;
        cr.set_line_width(1. / drawlen as f64);

        let slen = series.len();

        let onelen = drawlen as usize / slen;

        for i in 0..slen {
            let serie = &series[i];
            cr.set_source_rgb(
                serie.0.color.red(),
                serie.0.color.green(),
                serie.0.color.blue(),
            );
            let points: Vec<Point> = Self::scale(&serie.0, &serie.1)
                .into_iter()
                .take(onelen)
                .collect();
            for (j, ele) in points.iter().enumerate() {
                let x = ((j * slen + i) as f64 + 0.5) * interval as f64;
                cr.move_to(
                    x,
                    match serie.1 {
                        DrawDirection::TopDown => 0.05,
                        DrawDirection::DownTop => 0.95,
                    },
                );
                cr.line_to(x, ele.y);
            }
            cr.stroke().unwrap();
        }
    }

    fn draw_up_and_down(
        series: &Vec<(Series<E>, DrawDirection)>,
        line_width: Option<usize>,
        da: &gtk::DrawingArea,
        cr: &gdk::cairo::Context,
    ) {
        let alloc = da.allocation();

        let width = alloc.width();
        let height = alloc.height();

        cr.scale(width as f64, height as f64);

        cr.set_source_rgba(0. / 255.0, 0. / 255.0, 0. / 255.0, 0.0);
        cr.paint().unwrap();

        let line_width = line_width.unwrap_or(3);
        let interval = line_width as f64 / width as f64;
        let drawlen = width / line_width as i32;
        cr.set_line_width(interval);

        let slen = series.len();
        let oneheight = 1. / slen as f64;

        for i in 0..slen {
            let serie = &series[i];
            cr.set_source_rgb(
                serie.0.color.red(),
                serie.0.color.green(),
                serie.0.color.blue(),
            );
            let points: Vec<Point> = Self::scale(&serie.0, &serie.1)
                .into_iter()
                .take(drawlen as usize)
                .collect();
            let base_height = oneheight * i as f64;
            for (j, ele) in points.iter().enumerate() {
                let x = (j as f64 + 0.5) * interval as f64;
                let y = ele.y * oneheight * 0.8;
                cr.move_to(x, 1. - (base_height));
                cr.line_to(x, 1. - (base_height + oneheight * 0.05 + y));
            }
            cr.stroke().unwrap();
        }
    }

    pub fn draw_in_seconds(&self, secs: u32) {
        let drawing_area = self.drawing_area.clone();
        glib::timeout_add_seconds_local(secs, move || {
            drawing_area.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    fn draw(
        series: &Vec<(Series<E>, DrawDirection)>,
        line_width: Option<usize>,
        da: &gtk::DrawingArea,
        cr: &gdk::cairo::Context,
    ) {
        let alloc = da.allocation();

        let width = alloc.width();
        let height = alloc.height();

        cr.scale(width as f64, height as f64);

        cr.set_source_rgba(0. / 255.0, 0. / 255.0, 0. / 255.0, 0.0);
        cr.fill().unwrap();

        let line_width = (line_width.unwrap_or(1) as f64) / width as f64;
        cr.set_line_width(line_width);

        for serie in series {
            let interval = 1. / serie.0.ring.size as f64;

            let point_vec: Vec<Point> = Self::scale(&serie.0, &serie.1).into_iter().collect();

            if point_vec.len() <= 1 {
                continue;
            }

            let transform = |v| {
                let v = match serie.1 {
                    DrawDirection::TopDown => v as f64,
                    DrawDirection::DownTop => 1. - v as f64,
                };

                0.05 + v * 0.9
            };

            cr.move_to(0., transform(point_vec[0].y));
            cr.set_source_rgb(
                serie.0.color.red(),
                serie.0.color.green(),
                serie.0.color.blue(),
            );

            for (i, ele) in point_vec.iter().skip(1).enumerate() {
                cr.line_to(i as f64 * interval, transform(ele.y));
            }

            cr.stroke().unwrap();
        }
    }

    fn scale(series: &Series<E>, _draw_direction: &DrawDirection) -> Vec<Point> {
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
                    y: f64::min(1., sh),
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
            line_width: None,
        }
    }
}

pub struct ChartBuilder<E: Into<f64> + Clone> {
    height: Option<i32>,
    width: Option<i32>,
    series: Vec<(Series<E>, DrawDirection)>,
    line_type: Option<LineType>,
    line_width: Option<usize>,
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

    pub fn line_width(mut self, size: usize) -> Self {
        self.line_width.replace(size);

        self
    }

    pub fn line_type(mut self, line_type: LineType) -> Self {
        self.line_type.replace(line_type);

        self
    }

    pub fn build(self) -> Chart<E> {
        let line_type = match self.line_type {
            None => LineType::Line,
            Some(t) => t,
        };

        let mut chart = Chart::new(self.series, self.line_width, line_type);
        if let Some(h) = self.height {
            chart.drawing_box.set_height_request(h);
        }

        if let Some(w) = self.width {
            chart.drawing_box.set_width_request(w);
        }

        if let Some(mp) = self.line_width {
            chart.line_width.replace(mp);
        }

        chart
    }
}
