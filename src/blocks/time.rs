use crate::datahodler::channel::DualChannel;
use crate::statusbar::WidgetShareInfo;
use chinese_lunisolar_calendar::LunisolarDate;
use chrono::Timelike;
use chrono::{DateTime, Local};
use gdk::glib::Cast;
use glib::MainContext;
use gtk::prelude::LabelExt;
use gtk::traits::BoxExt;
use gtk::traits::StyleContextExt;
use gtk::traits::WidgetExt;
use std::cell::RefCell;

use super::Block;

#[derive(Clone)]
pub enum TimeIn {}

#[derive(Clone)]
pub enum TimeOut {
    Chinese((String, String, String)),
    Westen(String, String),
}

pub struct TimeBlock {
    dualchannel: DualChannel<TimeOut, TimeIn>,
}

impl TimeBlock {
    pub fn new() -> Self {
        Self {
            dualchannel: DualChannel::new(100),
        }
    }

    fn get_wes_time() -> (String, String, u32) {
        let now: DateTime<Local> = Local::now();

        (
            now.format("%y-%m-%d").to_string(),
            now.format("%H:%M:%S").to_string(),
            now.hour(),
        )
    }

    fn get_chinese_date() -> (String, String, String) {
        let now: DateTime<Local> = Local::now();

        let date = LunisolarDate::from_date(now);

        match date {
            Ok(date) => (
                date.to_lunar_year().to_string(),
                date.to_lunar_month().to_string(),
                date.to_lunar_day().to_string(),
            ),
            Err(_) => ("Unknown".to_string(), "Error".to_string(), "".into()),
        }
    }
}

impl Block for TimeBlock {
    type Out = TimeOut;

    type In = TimeIn;

    fn run(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();
        let hour = RefCell::new(0);

        glib::timeout_add_seconds_local(1, move || {
            let (d, t, h) = Self::get_wes_time();

            sender.send(TimeOut::Westen(d, t)).unwrap();

            let oldt = hour.replace(h);

            if oldt != h && h >= 11 {
                sender
                    .send(TimeOut::Chinese(Self::get_chinese_date()))
                    .unwrap();
            }

            glib::ControlFlow::Continue
        });

        Ok(())
    }

    fn widget(&self, share_info: &WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .valign(gtk::Align::Center)
            .vexpand(false)
            .build();

        let (_, cnm, cnd) = Self::get_chinese_date();
        let cn_date = gtk::Label::builder()
            .label(format!("{}\n{}", cnm, cnd))
            .vexpand(false)
            .build();
        cn_date.style_context().add_class("time-chinese");

        let cn_holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(false)
            .build();
        cn_holder.pack_start(&cn_date, false, false, 0);

        let wes_date = gtk::Label::builder()
            .label(format!(
                "{}\n{}",
                Self::get_wes_time().0,
                Self::get_wes_time().1
            ))
            .vexpand(false)
            .build();
        wes_date.style_context().add_class("time-date");

        holder.pack_end(&cn_holder, false, false, 0);
        holder.pack_start(&wes_date, false, false, 0);

        let mut mreceiver = self.dualchannel.get_out_receiver();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match mreceiver.recv().await {
                    Ok(msg) => {
                        match msg {
                            TimeOut::Chinese((_, m, d)) => {
                                cn_date.set_label(format!("{}\n{}", m, d).as_str());
                            }
                            TimeOut::Westen(d, t) => {
                                wes_date.set_label(format!("{}\n{}", d, t).as_str());
                                // wes_t.set_label(d.as_str());
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        });

        holder.upcast()
    }
}
