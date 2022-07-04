use std::fs::File;
use std::path::Path;
use std::io;
use std::io::BufRead;
use gtk::Button;
use gtk::traits::{ButtonExt, WidgetExt, StyleContextExt};
use regex::Regex;
use tokio::spawn;

use super::Module;

const NET_DEV: &str = "/proc/net/dev";

pub struct NetspeedModule {}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}


fn get_total_all() -> (u64, u64) {
    let mut total_download = 0;
    let mut total_upload = 0;

    if let Ok(lines) = read_lines(NET_DEV) {
        // Consumes the iterator, returns an (Optional) String
        for x in lines {
            if let Ok(line) = x {
                let fields = Regex::new(r"\s+").expect("Invalid regex")
                    .split(line.trim())
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>();
                // line.trim().split(" ").collect::<Vec<&str>>();
                if fields.len() <= 10 {
                    continue;
                }

                if ! Regex::new(r"^[0-9]+$").unwrap().is_match(&fields[1]) {
                    continue;
                }

                let cidb : u64 = fields[1].parse().unwrap();
                let diub : u64 = fields[9].parse().unwrap();

                let interface = &fields[0];
                if interface == "lo"
                // Created by python-based bandwidth manager "traffictoll".
                    || Regex::new(r"^ifb[0-9]+").unwrap().is_match(&interface)
                // Created by lxd container manager.
                    || Regex::new(r"^lxdbr[0-9]+").unwrap().is_match(&interface)
                    || Regex::new(r"^virbr[0-9]+").unwrap().is_match(&interface)
                    || Regex::new(r"^br[0-9]+").unwrap().is_match(&interface)
                    || Regex::new(r"^vnet[0-9]+").unwrap().is_match(&interface)
                    || Regex::new(r"^tun[0-9]+").unwrap().is_match(&interface)
                    || Regex::new(r"^tap[0-9]+").unwrap().is_match(&interface) {
                        continue;
                    }

                total_download = total_download + cidb;
                total_upload = total_upload + diub;
            }
        }
    }

    return (total_download, total_upload)
}

impl Module<Button> for NetspeedModule {
    fn into_widget(self) -> Button {
        let netspeed = gtk::Button::with_label("");
        netspeed.style_context().add_class("block");

        let mut total_download: u64;
        let mut total_upload: u64;
        (total_download, total_upload) = get_total_all();

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        spawn(async move {
            loop {
                let t1 : u64;
                let t2 : u64;
                (t1, t2) = get_total_all();

                let diff_download_bytes = t1 - total_download;
                let diff_upload_bytes = t2 - total_upload;

                total_download = t1;
                total_upload = t2;
                let _ = tx.send(format!("{}{} KiB/s",  diff_upload_bytes / 1024, diff_download_bytes / 1024));
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
        });

        {
            let netspeed = netspeed.clone();
            rx.attach(None, move |s| {
                netspeed.set_label(s.as_str());
                glib::Continue(true)
            });
        }

        netspeed
    }
}
