pub mod hyprclients;
pub mod hyprevents;

use async_std::io::prelude::BufReadExt;
use async_std::io::BufReader;
use async_std::os::unix::net::UnixStream;
use glib::{Cast, Continue, MainContext};

use std::collections::HashSet;

use hyprevents::ParsedEventType;
use std::io::BufRead;
use std::path::Path;
use std::str::FromStr;

use crate::utils;

use super::Module;
use crate::blocks::hyprstatus::hyprclients::{HyprClient, HyprWorkspace};
use gtk::prelude::GridExt;
use gtk::traits::WidgetExt;
use gtk::traits::{BoxExt, ButtonExt, StyleContextExt};

pub struct HyprStatus {}

pub enum ButtonType {
    Hide,
    Focus,
    Normal,
}

fn handle_events(
    receiver: glib::Receiver<ParsedEventType>,
    grid: &gtk::Grid,
    activate_window_button: &gtk::Button,
) {
    let mut icon_loader = utils::gtk_icon_loader::GtkIconLoader::new();

    let mut current_workspace: Option<String> = None;
    let mut current_monitor: Option<String> = None;
    let mut current_class: Option<String> = None;

    let grid = grid.clone();
    let activate_window_button = activate_window_button.clone();
    receiver.attach(None, move |event| {
        match event {
            ParsedEventType::WorkspaceChanged(ws) => {
                if let Some(cws_but) = grid.child_at(i32::from_str(ws.as_str()).unwrap(), 0) {
                    if let Ok(but) = cws_but.downcast::<gtk::Button>() {
                        change_ws_button(&but, ButtonType::Focus);
                    }
                } else {
                }

                if let Some(lws) = current_workspace.replace(ws) {
                    if let Some(lws_but) = grid.child_at(i32::from_str(lws.as_str()).unwrap(), 0) {
                        if let Ok(but) = lws_but.downcast::<gtk::Button>() {
                            change_ws_button(&but, ButtonType::Normal);
                        }
                    }
                }
            }
            ParsedEventType::WorkspaceDeleted(ws) => {
                let id = ws.parse::<i32>().unwrap();
                if let Some(cws_but) = grid.child_at(id, 0) {
                    if let Ok(but) = cws_but.downcast::<gtk::Button>() {
                        change_ws_button(&but, ButtonType::Hide);
                    }
                }
            }
            ParsedEventType::WorkspaceAdded(ws) => {
                let id = ws.parse::<i32>().unwrap();
                if let Some(cws_but) = grid.child_at(id.clone(), 0) {
                    if let Ok(but) = cws_but.downcast::<gtk::Button>() {
                        change_ws_button(&but, ButtonType::Normal);
                    }
                } else {
                    get_ws_button(
                        &grid,
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
                let visable = activate_window_button.get_visible();
                if class.is_empty() {
                    if visable {
                        activate_window_button.set_visible(false);
                    }
                } else {
                    if !visable {
                        activate_window_button.set_visible(true);
                    }
                    activate_window_button.set_label(title.as_str());
                    let c = class.clone();
                    if let Some(lc) = current_class.replace(class) {
                        if lc != c {
                            let image = icon_loader.load_from_name(c.as_str());
                            activate_window_button.set_image(image);
                        }
                    }
                }
            }
            ParsedEventType::ActiveMonitorChanged(monitor, _ws) => {
                current_monitor.replace(monitor);
            }
            _ => {}
        }
        Continue(true)
    });
}

async fn read_socket(tx: &glib::Sender<ParsedEventType>) {
    match std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        Ok(ins) => {
            let socket = format!("/tmp/hypr/{}/.socket2.sock", ins);
            let socket_path = Path::new(socket.as_str());

            println!("Listening on: {:?}", socket_path);
            let regexes = hyprevents::get_event_regex();

            while let Ok(mut stream) = UnixStream::connect(socket_path).await {
                println!("Accepted a new connection");

                let mut buffer = String::new();
                let mut reader = BufReader::new(&mut stream);

                while let Ok(bytes_read) = reader.read_line(&mut buffer).await {
                    if bytes_read == 0 {
                        break;
                    }

                    let event = hyprevents::convert_line_to_event(&regexes, buffer.as_str());
                    tx.send(event).unwrap();

                    buffer.clear();
                }
            }
        }
        Err(_e) => {}
    }
}

impl Module for HyprStatus {
    fn to_widget(&self) -> gtk::Widget {
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let full_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        full_container.style_context().add_class("wm");

        let active_window_button = get_active_window_button();
        full_container.pack_start(&active_window_button, false, false, 0);

        let ws_container = get_ws_container();
        full_container.pack_start(&ws_container, false, false, 0);

        handle_events(rx, &ws_container, &active_window_button);

        let clients = match hyprclients::get_clients() {
            Ok(vec) => vec,
            Err(_) => vec![],
        };

        let workspaces = clients
            .iter()
            .map(|e| e.workspace.clone())
            .collect::<HashSet<HyprWorkspace>>();

        let active_hypr_client: Option<HyprClient> = hyprclients::get_active_window_address()
            .and_then(|address| {
                clients.iter().find(|&c| c.address == address).cloned()
            });

        for x in workspaces {
            let _ = tx.send(ParsedEventType::ActiveMonitorChanged(
                x.monitor.to_string(),
                x.id.to_string(),
            ));
        }
        if let Some(active_client) = active_hypr_client {
            let workspace = &active_client.workspace;
            let _ = tx.send(ParsedEventType::ActiveMonitorChanged(
                workspace.monitor.to_string(),
                workspace.id.to_string(),
            ));

            let _ = tx.send(ParsedEventType::ActiveWindowChangedV1(
                active_client.class.to_string(),
                active_client.title.to_string(),
            ));
        }

        MainContext::ref_thread_default().spawn_local(async move {
            read_socket(&tx).await;
        });

        full_container.upcast()
    }

    fn put_into_bar(&self, bar: &gtk::Box) {
        bar.pack_start(&self.to_widget(), false, false, 0);
    }
}

fn get_ws_container() -> gtk::Grid {
    let ws_container = gtk::Grid::builder().build();
    ws_container.style_context().add_class("wss");

    ws_container
}

fn get_active_window_button() -> gtk::Button {
    let active_window = gtk::Button::builder().build();
    active_window.style_context().add_class("wm-title");

    active_window
}

fn create_ws_button(ws: &HyprWorkspace) -> gtk::Button {
    let label = ws.get_bar_name();

    let wb = gtk::Button::builder()
        .label(label)
        .name(&ws.id.to_string())
        .build();

    wb.style_context().add_class("ws");

    wb.connect_clicked(move |_| {});

    wb
}

fn get_ws_button(grid: &gtk::Grid, ws: &HyprWorkspace) -> gtk::Button {
    match grid.child_at(ws.id.clone() as i32, 0) {
        None => {
            let wb = create_ws_button(ws);
            grid.attach(&wb, ws.id.clone() as i32, 0, 1, 1);
            wb
        }
        Some(button) => button.downcast::<gtk::Button>().unwrap(),
    }
}

fn change_ws_button(button: &gtk::Button, msg: ButtonType) {
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
