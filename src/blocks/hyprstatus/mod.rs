pub mod hyprclients;
pub mod hyprevents;

use async_std::io::prelude::BufReadExt;
use async_std::io::BufReader;
use async_std::os::unix::net::UnixStream;
use glib::{Cast, MainContext};

use std::collections::HashSet;

use hyprevents::ParsedEventType;
use std::io::BufRead;
use std::path::Path;
use std::str::FromStr;

use crate::utils;

use gtk::prelude::GridExt;
use gtk::traits::WidgetExt;
use gtk::{
    traits::{BoxExt, ButtonExt, StyleContextExt},
    Button,
};

use crate::blocks::hyprstatus::hyprclients::HyprWorkspace;
use crate::blocks::hyprstatus::ButtonType::{Focus, Hide, Normal};

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

async fn read_msgs(
    grid: &gtk::Grid,
    activate_window: &gtk::Button,
    icon_loader: &mut utils::gtk_icon_loader::GtkIconLoader,
    monitor: &str,
) {
    match std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        Ok(ins) => {
            let socket = format!("/tmp/hypr/{}/.socket2.sock", ins);

            let socket_path = Path::new(socket.as_str());

            println!("Listening on: {:?}", socket_path);

            let regexes = hyprevents::get_event_regex();
            let mut current_workspace: Option<String> = None;
            let mut current_monitor: Option<String> = Some(monitor.to_string());
            let mut current_class: Option<String> = None;

            while let Ok(mut stream) = UnixStream::connect(socket_path).await {
                println!("Accepted a new connection");

                let mut buffer = String::new();
                let mut reader = BufReader::new(&mut stream);

                while let Ok(bytes_read) = reader.read_line(&mut buffer).await {
                    if bytes_read == 0 {
                        break; // End of stream
                    }

                    let event = hyprevents::convert_line_to_event(&regexes, buffer.as_str());
                    match event {
                        ParsedEventType::WorkspaceChanged(ws) => {
                            if let Some(cws_but) =
                                grid.child_at(i32::from_str(ws.as_str()).unwrap(), 0)
                            {
                                if let Ok(but) = cws_but.downcast::<Button>() {
                                    change_button(&but, Focus);
                                }
                            } else {
                            }

                            if let Some(lws) = current_workspace.replace(ws) {
                                if let Some(lws_but) =
                                    grid.child_at(i32::from_str(lws.as_str()).unwrap(), 0)
                                {
                                    if let Ok(but) = lws_but.downcast::<Button>() {
                                        change_button(&but, Normal);
                                    }
                                }
                            }
                        }
                        ParsedEventType::WorkspaceDeleted(ws) => {
                            let id = ws.parse::<i32>().unwrap();
                            if let Some(cws_but) = grid.child_at(id, 0) {
                                if let Ok(but) = cws_but.downcast::<Button>() {
                                    change_button(&but, Hide);
                                }
                            }
                        }
                        ParsedEventType::WorkspaceAdded(ws) => {
                            let id = ws.parse::<i32>().unwrap();
                            if let Some(cws_but) = grid.child_at(id.clone(), 0) {
                                if let Ok(but) = cws_but.downcast::<Button>() {
                                    change_button(&but, Normal);
                                }
                            } else {
                                get_workspace_button(
                                    grid,
                                    &HyprWorkspace {
                                        id: id as i64,
                                        monitor: current_monitor.clone().unwrap().to_string(),
                                        name: "".to_string(),
                                    },
                                );
                            }
                        }
                        ParsedEventType::WorkspaceMoved(_ws, _m) => {}
                        ParsedEventType::ActiveWindowChangedV1(class, title) => {
                            let visable = activate_window.get_visible();
                            if class.is_empty() {
                                if visable {
                                    activate_window.set_visible(false);
                                }
                            } else {
                                if !visable {
                                    activate_window.set_visible(true);
                                }
                                activate_window.set_label(title.as_str());
                                let c = class.clone();
                                if let Some(lc) = current_class.replace(class) {
                                    if lc != c {
                                        let image = icon_loader.load_from_name(c.as_str());
                                        activate_window.set_image(image);
                                    }
                                }
                            }
                        }
                        ParsedEventType::ActiveMonitorChanged(monitor, _ws) => {
                            current_monitor.replace(monitor);
                        }
                        _ => {}
                    }

                    buffer.clear();
                }
            }
        }
        Err(_e) => {}
    }
}

impl Module for HyprStatus {
    fn to_widget(&self) -> gtk::Widget {
        let mut icon_loader = utils::gtk_icon_loader::GtkIconLoader::new();

        let full_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        full_container.style_context().add_class("wm");

        let clients = match hyprclients::get_clients() {
            Ok(vec) => vec,
            Err(_) => vec![],
        };

        let id = match hyprclients::get_active_window() {
            None => "".to_string(),
            Some(id) => id.as_str().to_string(),
        };

        let vec = clients
            .iter()
            .filter_map(|c| {
                if c.address.eq(id.as_str()) {
                    Some((
                        c.class.as_str(),
                        c.title.as_str(),
                        c.workspace.id.clone(),
                        c.workspace.monitor.to_string(),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<(&str, &str, i64, String)>>();

        let (class, title, _wsid, monitor) = if vec.len() > 0 {
            vec.get(0).unwrap().clone()
        } else {
            ("", "", -1, "".to_string())
        };

        let ws_container = gtk::Grid::builder().build();
        ws_container.style_context().add_class("wss");
        full_container.pack_start(&ws_container, false, false, 0);

        let mut workspace_id_set: HashSet<i64> = HashSet::new();
        clients.iter().for_each(|w| {
            if !workspace_id_set.insert(w.workspace.id.clone()) {
                return;
            }

            let _ = get_workspace_button(&ws_container, &w.workspace);
        });

        let mut title = gtk::Button::builder().label(title);
        if let Some(image) = icon_loader.load_from_name(class) {
            title = title.image(image);
        }
        let active_window = title.visible(false).build();
        active_window.style_context().add_class("wm-title");
        full_container.pack_start(&active_window, false, false, 0);

        MainContext::ref_thread_default().spawn_local(async move {
            read_msgs(
                &ws_container,
                &active_window,
                &mut icon_loader,
                monitor.as_str(),
            )
            .await;
        });

        full_container.upcast()
    }

    fn put_into_bar(&self, bar: &gtk::Box) {
        bar.pack_start(&self.to_widget(), false, false, 0);
    }
}

fn create_workspace_button(ws: &HyprWorkspace) -> gtk::Button {
    let label = ws.get_bar_name();

    let wb = gtk::Button::builder()
        .label(label)
        .name(&ws.id.to_string())
        .build();

    wb.style_context().add_class("ws");

    wb.connect_clicked(move |_| {});

    wb
}

fn get_workspace_button(grid: &gtk::Grid, ws: &HyprWorkspace) -> Button {
    match grid.child_at(ws.id.clone() as i32, 0) {
        None => {
            let wb = create_workspace_button(ws);
            grid.attach(&wb, ws.id.clone() as i32, 0, 1, 1);
            wb
        }
        Some(button) => button.downcast::<Button>().unwrap(),
    }
}

pub enum ButtonType {
    Hide,
    Focus,
    Normal,
}

fn change_button(button: &Button, msg: ButtonType) {
    match msg {
        ButtonType::Hide => button.hide(),
        ButtonType::Focus => {
            let sc = button.style_context();
            if !sc.has_class("ws-focus") {
                sc.add_class("ws-focus");
            }
            if !button.is_visible() {
                button.show();
            }
        }
        ButtonType::Normal => {
            let sc = button.style_context();
            if sc.has_class("ws-focus") {
                sc.remove_class("ws-focus");
            }
            if !button.is_visible() {
                button.show();
            }
        }
    }
}
