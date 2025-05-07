use core::f64;
use std::marker::PhantomData;

use crate::prelude::*;



use crate::datahodler::ring::Ring;

#[derive(Clone, PartialEq, Eq)]
pub enum LineType {
    Fill,
}

#[derive(Clone, Debug)]
pub enum BaselineType {
    FixedPercent(f64),
    Upon,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct Column<E: Into<f64> + Clone> {
    id: String,
    threshold: E,
    ring: Ring<E>,
    color: RGBA,
    baseline_type: BaselineType,
    height_percent: f64,
    line_type: LineType,
}

impl<E: Into<f64> + Clone> Column<E> {
    pub fn new(id: &str, threshold: E, ring_size: usize, color: RGBA) -> Self {
        Column {
            id: id.to_string(),
            threshold,
            ring: Ring::new(ring_size),
            color,
            baseline_type: BaselineType::Upon,
            height_percent: 1.0,
            line_type: LineType::Fill,
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
    phondata: PhantomData<E>,
    pub drawing_box: gtk::Box,
    columns: Vec<Column<E>>,
}

impl<E: Into<f64> + Clone + 'static> Chart<E> {
    fn new() -> Self {
        let drawing_area = gtk::DrawingArea::builder()
            .vexpand(false)
            .hexpand(true)
            .build();

        let drawing_box = gtk::Box::builder().build();

        drawing_box.pack_start(&drawing_area, true, true, 0);
        drawing_box.style_context().add_class("chart");

        Self {
            drawing_area,
            line_width: 1.0,
            phondata: Default::default(),
            drawing_box,
            columns: vec![],
        }
    }

    pub fn draw_in_seconds(&self, secs: u32) {
        let columns = self.columns.clone();
        let line_width = self.line_width.clone();
        self.drawing_area.connect_draw(clone!(
            @strong columns,
            @strong line_width =>
            move |da, cr| {
                Self::draw(&columns, line_width, da, cr);
                Propagation::Proceed
            }
        ));

        let drawing_area = self.drawing_area.clone();
        timeout_add_seconds_local(secs, move || {
            drawing_area.queue_draw();

            ControlFlow::Continue
        });
    }

    fn draw(
        columns: &Vec<Column<E>>,
        line_width: f64,
        da: &DrawingArea,
        cr: &gtk::cairo::Context,
    ) {
        let alloc = da.allocation();

        let alloc_w = alloc.width();
        let alloc_h = alloc.height();

        cr.set_line_width(line_width);

        let max_column_size = columns.iter().map(|s| s.ring.size).max().unwrap_or(30);
        let interval = 1.0 / ((max_column_size - 2) as f64);

        let mut sum_heights_percent = vec![0.; alloc_w as usize + 2];
        let mut curx;

        let mut prev_alloc_ys: Option<Vec<(f64, f64)>> = None;

        for column in columns {
            let (ys, _max_y) = Self::scale(&column);

            if ys.len() <= 1 {
                continue;
            }

            curx = 0;

            let mut alloc_ys: Vec<(f64, f64)> = Vec::with_capacity(alloc_w as usize);
            let def_baseline = alloc_h;

            for (i, yt) in ys.iter().enumerate() {
                // get x and y
                let alloc_x = i as f64 * interval * alloc_w as f64;
                let base_percent = match column.baseline_type {
                    BaselineType::FixedPercent(base) => base,
                    BaselineType::Upon => sum_heights_percent[alloc_x as usize],
                };

                let y_percent = base_percent + yt * column.height_percent;

                // build sum_heights_percent
                if let BaselineType::Upon = column.baseline_type {
                    let last_x = curx;
                    let last_y = sum_heights_percent[last_x];

                    let step = if alloc_x as usize - curx > 0 {
                        (y_percent - last_y) / (alloc_x - curx as f64)
                    } else {
                        0.
                    };

                    for x in last_x..=(alloc_x as usize) {
                        sum_heights_percent[x] = last_y + (x - last_x) as f64 * step;
                        curx = x;
                    }

                    if last_x == 0 {
                        sum_heights_percent[0] = y_percent;
                    }
                }

                let alloc_x = alloc_w as f64 - alloc_x;

                let alloc_y = (1. - y_percent) * alloc_h as f64;
                alloc_ys.push((alloc_x, alloc_y));

                if i == 0 {
                    cr.move_to(alloc_x, alloc_y);
                    cr.set_source_rgb(
                        column.color.red(),
                        column.color.green(),
                        column.color.blue(),
                    );
                } else {
                    cr.line_to(alloc_x, alloc_y);
                }
            }

            if LineType::Fill == column.line_type {
                cr.stroke_preserve().unwrap();

                match column.baseline_type {
                    BaselineType::FixedPercent(baseline) => {
                        let base = 1. - baseline;
                        if let Some((x, _)) = alloc_ys.last() {
                            cr.line_to(*x, alloc_h as f64 * base);
                        }
                        if let Some((x, _)) = alloc_ys.first() {
                            cr.line_to(*x, alloc_h as f64 * base);
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
                    column.color.red(),
                    column.color.green(),
                    column.color.blue(),
                    column.color.alpha(),
                );
                cr.fill().unwrap();
            } else {
                cr.stroke().unwrap();
            }

            prev_alloc_ys.replace(alloc_ys);
        }
    }

    fn scale(column: &Column<E>) -> (Vec<f64>, f64) {
        let originals: Vec<f64> = column
            .ring
            .get_all()
            .into_iter()
            .map(|e| e.into())
            .collect();

        let threshold_def: f64 = column.threshold.clone().into();
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

    pub fn with_columns(mut self, columns: Column<E>) -> Self {
        if !self.columns.is_empty() {
            if self.columns[0].ring.size != columns.ring.size {
                tracing::warn!("the columns should have same sizes.");
            }
        }

        self.columns.push(columns);
        self
    }

    pub(crate) fn builder() -> Self {
        Self::new()
    }
}
