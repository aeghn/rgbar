use crate::datahodler::channel::DualChannel;
use crate::prelude::*;
use crate::statusbar::WidgetShareInfo;
use chinese_lunisolar_calendar::LunisolarDate;
use chrono::Timelike;
use chrono::{DateTime, Local};
use glib::MainContext;
use gtk::prelude::LabelExt;
use gtk::traits::StyleContextExt;
use gtk::traits::WidgetExt;
use std::cell::RefCell;

use super::Block;

#[derive(Clone)]
pub enum TimeIn {}

#[derive(Clone)]
pub enum TimeOut {
    Chinese {
        year: String,
        month: String,
        day: String,
    },
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
            now.format("%m-%d %a").to_string(),
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
                let d = Self::get_chinese_date();
                sender
                    .send(TimeOut::Chinese {
                        year: d.0,
                        month: d.1,
                        day: d.2,
                    })
                    .unwrap();
            }

            glib::ControlFlow::Continue
        });

        Ok(())
    }

    fn widget(&self, _share_info: &WidgetShareInfo) -> gtk::Widget {
        let wes = Self::get_wes_time();
        let date_container = gtk::Label::builder()
            .label(format!("{} {}", wes.0, wes.1))
            .vexpand(false)
            .build();
        date_container.style_context().add_class("time-date");

        {
            let wes_date = date_container.clone();
            let mut mreceiver = self.dualchannel.get_out_receiver();
            MainContext::ref_thread_default().spawn_local(async move {
                loop {
                    match mreceiver.recv().await {
                        Ok(msg) => match msg {
                            TimeOut::Chinese { year, month, day } => {
                                let cn_date = format!("{year}年 {month} {day}");
                                wes_date.set_tooltip_text(Some(cn_date.as_str()));
                            }
                            TimeOut::Westen(d, t) => {
                                wes_date.set_label(format!("{} {}", d, t).as_str());
                            }
                        },
                        Err(_) => {}
                    }
                }
            });
        }

        date_container.upcast()
    }
}
