use gdk::RGBA;
use glib::{Continue, MainContext, Cast};
use gtk::prelude::{GridExt, LabelExt};
use human_bytes::human_bytes;
use regex::Regex;


use crate::datahodler::channel::{DualChannel, SSender, MReceiver};
use crate::utils::fileutils;
use crate::widgets::chart::{Chart, Series, SeriesType};

use super::Block;

const NET_DEV: &str = "/proc/net/dev";

#[derive(Clone)]
pub enum NetspeedBM {}

#[derive(Clone)]
pub enum NetspeedWM {
    NetspeedDiff(f64, f64),
}

pub struct NetspeedBlock {
    dualchannel: DualChannel<NetspeedWM, NetspeedBM>
}

impl NetspeedBlock {
    pub fn new() -> Self {
        let dualchannel = DualChannel::new(100);

        NetspeedBlock {
            dualchannel
        }
    }

    fn read_total_bytes() -> (u64, u64) {
        let mut total_download = 0;
        let mut total_upload = 0;

        if let Ok(lines) = fileutils::read_lines(NET_DEV) {
            // Consumes the iterator, returns an (Optional) String
            for x in lines {
                if let Ok(line) = x {
                    let fields = Regex::new(r"\s+")
                        .expect("Invalid regex")
                        .split(line.trim())
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>();
                    // line.trim().split(" ").collect::<Vec<&str>>();
                    if fields.len() <= 10 {
                        continue;
                    }

                    if !Regex::new(r"^[0-9]+$").unwrap().is_match(&fields[1]) {
                        continue;
                    }

                    let cidb: u64 = fields[1].parse().unwrap();
                    let diub: u64 = fields[9].parse().unwrap();

                    let interface = &fields[0];
                    if Regex::new(r"^lo:?").unwrap().is_match(&interface)
                // Created by python-based bandwidth manager "traffictoll".
                    || Regex::new(r"^ifb[0-9]+:?").unwrap().is_match(&interface)
                // Created by lxd container manager.
                    || Regex::new(r"^lxdbr[0-9]+:?").unwrap().is_match(&interface)
                    || Regex::new(r"^virbr[0-9]+:?").unwrap().is_match(&interface)
                    || Regex::new(r"^br[0-9]+:?").unwrap().is_match(&interface)
                    || Regex::new(r"^vnet[0-9]+:?").unwrap().is_match(&interface)
                    || Regex::new(r"^tun[0-9]+:?").unwrap().is_match(&interface)
                    || Regex::new(r"^tap[0-9]+:?").unwrap().is_match(&interface)
                    {
                        continue;
                    }

                    total_download = total_download + cidb;
                    total_upload = total_upload + diub;
                }
            }
        }

        return (total_download, total_upload);
    }
}

impl Block for NetspeedBlock {
    type BM = NetspeedBM;
    type WM = NetspeedWM;

    fn loop_receive(&mut self) {
        let (mut last_total_download, mut last_total_upload) = Self::read_total_bytes();
        let mut last_update_time = None;

        let sender = self.dualchannel.get_out_sender();

        glib::timeout_add_seconds_local(1, move || {
            let (download, upload) = Self::read_total_bytes();
            let now = std::time::SystemTime::now();
            if let Some(last) = last_update_time.replace(now) {
                let diff_download_bytes = download - last_total_download;
                let diff_upload_bytes = upload - last_total_upload;

                let diff = now.duration_since(last);
                if let Ok(dur) = diff {
                    last_total_download = download;
                    last_total_upload = upload;

                    let secs = dur.as_millis() as f64 / 1000.;

                    let convert = |bytes: u64| ->  f64 { bytes as f64 / secs };

                    sender.send(Self::WM::NetspeedDiff(convert(diff_upload_bytes), convert(diff_download_bytes)));
                }
            }

            Continue(true)
        });
    }

    fn get_channel(&self) -> (&SSender<Self::BM>, &MReceiver<Self::WM>) {
        self.dualchannel.get_reveled()
    }

    fn widget(&self) -> gtk::Widget {
        let holder = gtk::Grid::builder().margin(5).build();
        
        let uplabel = gtk::Label::builder().build();
        let downlabel = gtk::Label::builder().build();

        let up_series = Series::new("up", 5_000_000., 60, RGBA::new(0.4, 0.4, 0.4, 1.0), SeriesType::Line, true);
        let down_series = Series::new("down", 5_000_000., 60, RGBA::new(0.9, 0.9, 0.9, 1.0), SeriesType::Line, true);
        let chart = Chart::builder().height(20).width(60)
        .with_series(up_series.clone())
        .with_series(down_series.clone()).build();

        holder.attach(&uplabel, 0, 0, 1, 1);
        holder.attach(&downlabel, 1, 0, 1, 1);
        holder.attach(&chart.drawing_area, 0, 1, 1, 1);


        let mut mreceiver = self.dualchannel.get_out_receiver();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match mreceiver.recv().await {
                    Ok(msg) => {
                        match msg {
                            NetspeedWM::NetspeedDiff(up, down) => {
                                up_series.add_value(up.clone());
                                down_series.add_value(down.clone());
                                uplabel.set_label(format!("UP: {} ", human_bytes(up)).as_str());
                                downlabel.set_label(format!("DOWN: {} ", human_bytes(down)).as_str());
                            }
                            
                        }
                    },
                    Err(_) => {},
                }
            }
        });

        holder.upcast()
    }
}