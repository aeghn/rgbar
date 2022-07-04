use std::cmp::Ordering;

use gtk::{traits::{BoxExt, ButtonExt, ContainerExt, StyleContextExt, LabelExt}, Widget, Label};
use gtk::traits::WidgetExt;
use hyprland::{data::{Workspaces, Monitor}, event_listener::WindowEventData};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial};
use hyprland::prelude::*;
use hyprland::event_listener;
use hyprland::shared::WorkspaceType;
use tokio::spawn;
use tracing::info;

use super::Module;

pub struct HyprStatus {

}

#[derive(Debug, Clone)]
pub struct HWS {
    pub name: String,
}

pub struct HC {
    pub name: String,
}


pub enum HyprlandEvent {
    WSAdd(HWS, String),
    WSRemove(HWS),
    WSChange(HWS),
    CChange(HC),
}


impl Module<gtk::Box> for HyprStatus {
    fn into_widget(self) -> gtk::Box {
        let workspaces = Workspaces::get().unwrap();

        let full_container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        full_container.style_context().add_class("wm");

        let ws_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ws_container.style_context().add_class("wss");


        let title = Label::new(Some(""));
        title.style_context().add_class("wm-title");
        full_container.pack_start(&ws_container, false, false, 0);
        full_container.pack_start(&title, false, false, 0);

        for w in workspaces {
            let wb = create_workspace_button(w.name.to_string(), w.monitor.to_string());
            ws_container.pack_start(&wb, false, false, 0);
        }
        reorder_workspaces(&ws_container);

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        spawn(async move {
            let mut el = event_listener::EventListener::new();
            {
                let tx = tx.clone();
                el.add_workspace_added_handler(move |wst| {
                    let wsn = get_workspace_name(wst);
                    let current_monitor = Monitor::get_active().ok().unwrap().name;
                    tx.send(HyprlandEvent::WSAdd(HWS{name: wsn}, current_monitor)).unwrap();
                });
            }

            {
                let tx = tx.clone();
                el.add_workspace_destroy_handler(move |wst| {
                    let wsn = get_workspace_name(wst);
                    tx.send(HyprlandEvent::WSRemove(HWS{name: wsn})).unwrap();
                });
            }
            {
                let tx = tx.clone();
                el.add_workspace_change_handler(move |wst| {
                    let wsn = get_workspace_name(wst);
                    tx.send(HyprlandEvent::WSChange(HWS{name: wsn})).unwrap();
                });
            }

            {
                let tx = tx.clone();
                el.add_active_window_changed_handler(move |wed| {
                    let wsn = get_client_name(&wed);
                    tx.send(HyprlandEvent::CChange(HC{name: wsn.to_string()})).unwrap();
                });
            }

            {
                let tx = tx.clone();
                el.add_workspace_moved_handler(move |med| {
                    let wsn = get_workspace_name(med.workspace);
                    tx.send(HyprlandEvent::WSChange(HWS{name: wsn})).unwrap();
                });
            }

            {
                let tx = tx.clone();
                el.add_active_monitor_change_handler(move |med| {
                    let wsn = get_workspace_name(med.workspace);
                    tx.send(HyprlandEvent::WSChange(HWS{name: wsn})).unwrap();
                });
            }



            el.start_listener().unwrap();
        });



        let mut last: Option<Widget> = None;

        {
            let container = ws_container.clone();
            let title = title.clone();
            rx.attach(None, move |we| {
                match we {
                    HyprlandEvent::WSAdd(ws, monitor) => {
                        let ws = create_workspace_button(ws.name.to_string(), monitor);
                        container.add(&ws);
                        ws.show();
                    }
                    HyprlandEvent::WSRemove(ws) => {
                        let cc = container.children();
                        for c in cc {
                            let c = c.clone();
                            if c.widget_name() == ws.name {
                                container.remove(&c);
                            }
                        }
                    },
                    HyprlandEvent::WSChange(ws) => {
                        let cc = container.children();
                        {
                            match &last {
                                Some(l) => {
                                    let sc = l.style_context();
                                    if sc.has_class("ws-focus") {
                                        sc.remove_class("ws-focus");
                                    }
                                },
                                None => {},
                            }

                        }
                        for c in cc {
                            let c = c.clone();
                            if c.widget_name() == ws.name {
                                let sc = c.style_context();
                                if !sc.has_class("ws-focus") {
                                    sc.add_class("ws-focus");
                                }
                                last = Some(c);
                            }

                        }
                    },
                    HyprlandEvent::CChange(cn) => {
                        title.set_text(&cn.name);
                    },
                }
                reorder_workspaces(&container);
                glib::Continue(true)
            });
        }

        full_container
    }
}

fn get_workspace_name(name: WorkspaceType) -> String {
    match name {
        WorkspaceType::Regular(name) => name,
        WorkspaceType::Special(name) => name.unwrap_or_default(),
    }
}

fn create_workspace_button(name: String, monitor: String) -> gtk::Button {
    let mut label = name.clone();
    if monitor != "eDP-1" {
        info!("{:?}, {:?}", name, monitor);
        label.push_str(&monitor.as_str()[0..1]);
    }
    let wb = gtk::Button::with_label(&label);

    wb.set_widget_name(&name);
    wb.style_context().add_class("ws");

    wb.connect_clicked(move |_| {
        match Dispatch::call(DispatchType::Workspace(
            WorkspaceIdentifierWithSpecial::Name(name.as_str()),
        )) {
            Ok(x) => {
                println!("finished: {:?}", x);
            },
            Err(_) => todo!(),
        }
    });

    wb
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

fn get_client_name(wed: &Option<WindowEventData>) -> &str {
    let default = "";
    match wed {
        Some(w) => {
            w.window_title.as_str()
        },
        None => default
    }
}
