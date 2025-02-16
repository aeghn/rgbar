use crate::prelude::*;
use gdk::RGBA;
use glib::MainContext;

use gtk::prelude::{ContainerExt, LabelExt, StyleContextExt, WidgetExt};
use human_bytes::human_bytes;
use regex::Regex;

use crate::statusbar::WidgetShareInfo;
use crate::util::gtk_icon_loader::IconName;
use crate::util::{fileutil, gtk_icon_loader};
use crate::widgets::chart::{BaselineType, Chart, Column};

use super::Block;

const NET_DEV: &str = "/proc/net/dev";

#[derive(Clone)]
pub enum NetspeedIn {}

#[derive(Clone)]
pub enum NetspeedOut {
    NetspeedDiff(f64, f64),
}

pub struct NetspeedBlock {
    dualchannel: DualChannel<NetspeedOut, NetspeedIn>,
}

impl NetspeedBlock {
    pub fn new() -> Self {
        let dualchannel = DualChannel::new(100);

        NetspeedBlock { dualchannel }
    }

    fn read_total_bytes() -> (u64, u64) {
        let mut total_download = 0;
        let mut total_upload = 0;

        if let Ok(lines) = fileutil::read_lines(NET_DEV) {
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
                    if Regex::new(r"^lo:?").unwrap().is_match(&interface) ||
                        // Created by python-based bandwidth manager "traffictoll".
                        Regex::new(r"^ifb[0-9]+:?").unwrap().is_match(&interface) ||
                        // Created by lxd container manager.
                        Regex::new(r"^lxdbr[0-9]+:?").unwrap().is_match(&interface) ||
                        Regex::new(r"^virbr[0-9]+:?").unwrap().is_match(&interface) ||
                        Regex::new(r"^br[0-9]+:?").unwrap().is_match(&interface) ||
                        Regex::new(r"^vnet[0-9]+:?").unwrap().is_match(&interface) ||
                        Regex::new(r"^tun[0-9]+:?").unwrap().is_match(&interface) ||
                        Regex::new(r"^tap[0-9]+:?").unwrap().is_match(&interface)
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
    type In = NetspeedIn;
    type Out = NetspeedOut;

    fn run(&mut self) -> anyhow::Result<()> {
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

                    let secs = (dur.as_millis() as f64) / 1000.0;

                    let convert = |bytes: u64| -> f64 { (bytes as f64) / secs };

                    sender
                        .send(Self::Out::NetspeedDiff(
                            convert(diff_upload_bytes),
                            convert(diff_download_bytes),
                        ))
                        .unwrap();
                }
            }

            glib::ControlFlow::Continue
        });

        Ok(())
    }

    fn widget(&self, _share_info: &WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(false)
            .build();

        let icon = gtk_icon_loader::load_icon(IconName::WIFI);

        let speed_label: gtk::Label = gtk::Label::builder().hexpand(false).xalign(1.0).build();
        speed_label.style_context().add_class("netspeed-label");

        let up_color = RGBA::new(0.4, 0.4, 0.2, 0.6);
        let down_color = RGBA::new(0.3, 0.3, 0.8, 0.6);
        let up_columns = Column::new("up", 2_000_000.0, 60, up_color.clone())
            .with_baseline(BaselineType::FixedPercent(0.5))
            .with_height_percent(0.50);
        let down_columns = Column::new("down", 2_000_000.0, 60, down_color.clone())
            .with_baseline(BaselineType::FixedPercent(0.48))
            .with_height_percent(-0.45);

        let chart = Chart::builder()
            .with_line_width(1.0)
            .with_width(60)
            .with_columns(down_columns.clone())
            .with_columns(up_columns.clone());

        chart.draw_in_seconds(1);

        holder.add(&icon);
        holder.add(&chart.drawing_box);

        holder.add(&speed_label);

        let mut mreceiver = self.dualchannel.get_out_receiver();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match mreceiver.recv().await {
                    Ok(msg) => match msg {
                        NetspeedOut::NetspeedDiff(up, down) => {
                            up_columns.add_value(up.clone());
                            down_columns.add_value(down.clone());
                            speed_label.set_label(
                                format!("{}\n{} ", human_bytes(up), human_bytes(down)).as_str(),
                            );
                        }
                    },
                    Err(_) => {}
                }
            }
        });

        holder.upcast()
    }
}
