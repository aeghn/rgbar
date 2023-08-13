pub mod hyprclients;
pub mod hyprevents;

use async_std::io::prelude::BufReadExt;
use async_std::io::{BufReader, ReadExt};
use async_std::os::unix::net::UnixListener;
use glib::{Cast, MainContext};
use std::cmp::Ordering;
use std::env::VarError;
use std::io::BufRead;
use std::path::Path;

use crate::blocks::hyprstatus::hyprclients::HyprWindowResult;
use crate::utils;
use crate::utils::gtk_icon_loader;
use gtk::atk::Role::Label;
use gtk::traits::WidgetExt;
use gtk::{
    traits::{BoxExt, ButtonExt, ContainerExt, StyleContextExt},
    Image, Widget,
};
use tracing::info;

use super::Module;

pub struct HyprStatus {}

#[derive(Debug, Clone)]
pub struct HWS {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct HC {
    pub class: String,
    pub name: String,
}

pub enum HyprlandEvent {
    WSAdd(HWS, String),
    WSRemove(HWS),
    WSChange(HWS),
    CChange(HC),
}

async fn read_msgs() {
    match std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        Ok(ins) => {
            let socket = format!("/tmp/hypr/{}/.socket2.sock", ins);

            let socket_path = Path::new(socket.as_str());
            let listener = UnixListener::bind(socket_path).await.unwrap();

            println!("Listening on: {:?}", socket_path);

            while let Ok((mut stream, _)) = listener.accept().await {
                println!("Accepted a new connection");

                let mut buffer = String::new();
                let mut reader = BufReader::new(&mut stream);

                while let Ok(bytes_read) = reader.read_line(&mut buffer).await {
                    if bytes_read == 0 {
                        break; // End of stream
                    }

                    // Process the received line
                    println!("Received line: {}", buffer.trim());

                    buffer.clear();
                }
            }
        }
        Err(e) => {}
    }
}

impl Module for HyprStatus {
    fn to_widget(&self) -> gtk::Widget {
        let full_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        full_container.style_context().add_class("wm");

        let ws_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ws_container.style_context().add_class("wss");

        let mut icon_loader = utils::gtk_icon_loader::GtkIconLoader::new();

        let clients = hyprclients::get_clients();
        let clients = match clients {
            Ok(vec) => vec,
            Err(_) => {
                vec![]
            }
        };

        let id = match hyprclients::get_active_window() {
            None => "".to_string(),
            Some(id) => id.as_str().to_string(),
        };

        let vec = clients
            .iter()
            .filter_map(|c| {
                if c.address.eq(id.as_str()) {
                    Some((c.class.as_str(), c.title.as_str(), c.workspace.id))
                } else {
                    None
                }
            })
            .collect::<Vec<(&str, &str, i64)>>();

        let (class, title, wsid) = if vec.len() > 0 {
            vec.get(0).unwrap().clone()
        } else {
            ("", "", -1)
        };

        MainContext::ref_thread_default().spawn_local(async move {
            read_msgs().await;
        });

        if (2 > 1) {
            return gtk::Label::new(Some(&"adasdasd")).upcast();
        }

        let mut title = gtk::Button::builder().label(title);
        if let Some(image) = icon_loader.load_from_name(class) {
            title = title.image(image);
        }
        let title = title.build();

        title.style_context().add_class("wm-title");
        full_container.pack_start(&ws_container, false, false, 0);
        full_container.pack_start(&title, false, false, 0);
        title.set_visible(false);

        // for w in workspaces {
        //     let wb = create_workspace_button(w.name.to_string(), w.monitor.to_string());
        //     ws_container.pack_start(&wb, false, false, 0);
        // }
        // reorder_workspaces(&ws_container);

        // let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        return gtk::Label::new(Some(&"adasdasd")).upcast();
    }

    fn put_into_bar(&self, bar: &gtk::Box) {
        bar.pack_start(&self.to_widget(), false, false, 0);
    }
}

fn reorder_workspaces(wbb: &gtk::Box) {
    let mut buttons = wbb
        .children()
        .into_iter()
        .map(|child| (child.widget_name().to_string(), child))
        .collect::<Vec<_>>();
    buttons.sort_by(|(label_a, _), (label_b, _a)| {
        match (label_a.parse::<i32>(), label_b.parse::<i32>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => label_a.cmp(label_b),
        }
    });

    for (i, (_, button)) in buttons.into_iter().enumerate() {
        wbb.reorder_child(&button, i as i32);
    }
}
