use std::cell::RefCell;
use chinese_lunisolar_calendar::LunisolarDate;
use chrono::Timelike;
use chrono::{DateTime, Local};
use glib::{Continue, Cast, MainContext};
use gtk::prelude::{OrientableExt, LabelExt};
use gtk::traits::BoxExt;
use gtk::traits::ButtonExt;
use gtk::traits::StyleContextExt;
use gtk::traits::WidgetExt;
use tracing_subscriber::fmt::format;
use crate::datahodler::channel::DualChannel;

use super::Block;

#[derive(Clone)]
pub enum TimeBM {
    
}

#[derive(Clone)]
pub enum TimeWM {
    Chinese((String, String, String)),
    Westen(String, String)
}

pub struct TimeBlock {
    dualchannel: DualChannel<TimeWM, TimeBM> 
}

impl TimeBlock {
    pub fn new() -> Self {
        Self {
            dualchannel: DualChannel::new(100)
        }
    }

    fn get_wes_time() -> (String, String, u32) {
        let now: DateTime<Local> = Local::now();
        
        (now.format("%y-%m-%d").to_string(), now.format("%H:%M:%S").to_string(), now.hour())
    }
    
    fn get_chinese_date() -> (String, String, String) {
        let now: DateTime<Local> = Local::now();

        let date = LunisolarDate::from_date(now);

        match date {
            Ok(date) => {
                (date.to_lunar_year().to_string(), date.to_lunar_month().to_string(), date.to_lunar_day().to_string())
            },
            Err(_) => ("Unknown".to_string(), "Error".to_string(), "".into()),
        }
    }
}

impl Block for TimeBlock {
    type WM = TimeWM;

    type BM = TimeBM;

    fn loop_receive(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();
        let hour = RefCell::new(0);

        glib::timeout_add_seconds_local(1, move || {
            let (d, t, h) = Self::get_wes_time();

            sender.send(TimeWM::Westen(d, t));

            let oldt = hour.replace(h);

            if oldt != h && h >= 11 {
                sender.send(TimeWM::Chinese(Self::get_chinese_date()));
            }

            Continue(true)
        });

        Ok(())
    }

    fn get_channel(&self) -> (&crate::datahodler::channel::SSender<Self::BM>, &crate::datahodler::channel::MReceiver<Self::WM>) {
        self.dualchannel.get_reveled()
    }

    fn widget(&self) -> gtk::Widget {
        let (cny, cnm, cnd) = Self::get_chinese_date();
        let cn_y = gtk::Label::builder().label(format!("({}) {} {}", cny, cnm, cnd)).build();
        cn_y.style_context().add_class("time-chinese");

        let cn_holder = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
        cn_holder.pack_start(&cn_y, false, false, 0);

        let wes_d = gtk::Label::builder().label(format!("{} {}", Self::get_wes_time().0, Self::get_wes_time().1)).build();
        wes_d.style_context().add_class("time-date");
        // let wes_t = gtk::Label::builder().label(Self::get_wes_time().1).build();
        // wes_t.style_context().add_class("time-time");
        // let wes_holder = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
        // wes_holder.pack_start(&wes_d, false, false, 0);
        // wes_holder.pack_start(&wes_t, false, false, 0);

        let holder = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
        holder.pack_start(&cn_holder, false, false, 0);
        holder.pack_start(&wes_d, false, false, 0);

        holder.style_context().add_class("block");

        let mut mreceiver = self.dualchannel.get_out_receiver();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match mreceiver.recv().await {
                    Ok(msg) => {
                        match msg {
                            TimeWM::Chinese((y, m, d)) => {
                                cn_y.set_label(format!("({}) {} {}", y, m, d).as_str());
                            },
                            TimeWM::Westen(t, d) => {
                                wes_d.set_label(format!("{} {}", Self::get_wes_time().0, Self::get_wes_time().1).as_str());
                                // wes_t.set_label(d.as_str());
                            },
                        }
                    },
                    Err(_) => {},
                }
            }
        });        

        holder.upcast()
    }
}