use core::f64;
use std::marker::PhantomData;

use gdk::glib::Propagation;
use gdk::RGBA;
use gtk::prelude::BoxExt;

use gtk::prelude::StyleContextExt;
use gtk::prelude::WidgetExt;


use crate::datahodler::ring::Ring;

#[derive(Clone)]
pub enum LineType {
    Line,
    Pillar,
}

#[derive(Clone, Debug)]
pub enum BaselineType {
    FixedPercent(f64),
    Upon,
}

#[derive(Clone)]
pub struct Series<E: Into<f64> + Clone> {
    id: String,
    threshold: E,
    ring: Ring<E>,
    color: RGBA,
    baseline_type: BaselineType,
    height_percent: f64,
}

impl<E: Into<f64> + Clone> Series<E> {
    pub fn new(id: &str, threshold: E, ring_size: usize, color: RGBA) -> Self {
        Series {
            id: id.to_string(),
            threshold,
            ring: Ring::new(ring_size),
            color,
            baseline_type: BaselineType::Upon,
            height_percent: 1.0,
        }
    }

    pub fn with_baseline(mut self, baseline_type: BaselineType) -> Self {
        self.baseline_type = baseline_type;
        self
    }

    pub fn with_height_percent(mut self, height_percent: f64) -> Self {
        self.height_percent = height_percent;
        self
    }

    pub fn add_value(&self, value: E) {
        self.ring.add(value);
    }
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
        drawing_box.style_context().add_class("chart-border");

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

        let alloc_w = alloc.width();
        let alloc_h = alloc.height();

        cr.set_line_width(line_width);

        let max_serie_size = series.iter().map(|s| s.ring.size).max().unwrap_or(30);
        let interval = 1.0 / ((max_serie_size - 2) as f64);

        let mut sum_heights_percent = vec![0.; max_serie_size + 2];
        let mut cur_x = 0;

        let mut prev_alloc_ys: Option<Vec<(f64, f64)>> = None;

        for serie in series {
            let (ys, _max_y) = Self::scale(&serie);

            if ys.len() <= 1 {
                continue;
            }

            cur_x = 0;

            let mut alloc_ys: Vec<(f64, f64)> = Vec::with_capacity(alloc_w as usize);
            let def_baseline = alloc_h;

            for (i, yt) in ys.iter().enumerate() {
                // get x and y
                let alloc_x = i as f64 * interval * alloc_w as f64;
                let base_percent = match serie.baseline_type {
                    BaselineType::FixedPercent(base) => base,
                    BaselineType::Upon => sum_heights_percent[alloc_x as usize],
                };

                let y_percent = base_percent + yt * serie.height_percent;

                // build sum_heights_percent
                if let BaselineType::Upon = serie.baseline_type {
                    let last_x = cur_x;
                    let last_y = sum_heights_percent[last_x];

                    let step = if alloc_x as usize - cur_x > 0 {
                        (y_percent - last_y) / (alloc_x - cur_x as f64)
                    } else {
                        0.
                    };

                    for x in last_x..=(alloc_x as usize) {
                        sum_heights_percent[x] = last_y + (x - last_x) as f64 * step;
                        cur_x = x;
                    }

                    if last_x == 0 {
                        sum_heights_percent[0] = y_percent;
                    }
                }

                let alloc_y = (1. - y_percent) * alloc_h as f64;
                alloc_ys.push((alloc_x, alloc_y));

                if i == 0 {
                    cr.move_to(alloc_x, alloc_y);
                    cr.set_source_rgb(serie.color.red(), serie.color.green(), serie.color.blue());
                } else {
                    cr.line_to(alloc_x, alloc_y);
                }
            }

            cr.stroke_preserve().unwrap();

            match serie.baseline_type {
                BaselineType::FixedPercent(baseline) => {
                    if let Some((x, _)) = alloc_ys.last() {
                        cr.line_to(*x, alloc_h as f64 * baseline);
                    }
                    if let Some((x, _)) = alloc_ys.first() {
                        cr.line_to(*x, alloc_h as f64 * baseline);
                    }
                }
                BaselineType::Upon => {
                    if let Some(vec) = prev_alloc_ys.as_ref() {
                        for (x, y) in vec.iter().rev() {
                            cr.line_to(*x, *y);
                        }
                    } else {
                        if let Some((x, _)) = alloc_ys.last() {
                            cr.line_to(*x, def_baseline as f64);
                        }
                        if let Some((x, _)) = alloc_ys.first() {
                            cr.line_to(*x, def_baseline as f64);
                        }
                    }

                    if let Some((x, y)) = alloc_ys.get(0) {
                        cr.line_to(*x, *y);
                    }
                }
            }

            cr.set_source_rgba(
                serie.color.red(),
                serie.color.green(),
                serie.color.blue(),
                serie.color.alpha(),
            );
            cr.fill().unwrap();

            prev_alloc_ys.replace(alloc_ys);
        }
    }

    fn scale(serie: &Series<E>) -> (Vec<f64>, f64) {
        let originals: Vec<f64> = serie.ring.get_all().into_iter().map(|e| e.into()).collect();

        let threshold_def: f64 = serie.threshold.clone().into();
        let mut true_threshold = threshold_def;

        for h in originals.as_slice() {
            true_threshold = f64::max(*h, true_threshold);
        }

        let vec = originals
            .into_iter()
            .rev()
            .map(|h| {
                let sh = h / true_threshold;
                f64::min(1.0, sh)
            })
            .collect();

        (vec, true_threshold)
    }

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
