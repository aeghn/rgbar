pub mod hyprclients;
pub mod hyprevents;

use crate::utils;
use anyhow::anyhow;
use gio::prelude::{DataInputStreamExtManual, IOStreamExtManual};
use gio::traits::SocketClientExt;
use gio::{DataInputStream, SocketClient};
use glib::{Cast, MainContext, Priority};
use hyprevents::ParsedEventType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

use self::hyprclients::HyprMonitor;

use super::Block;
use crate::blocks::hyprstatus::hyprclients::{get_monitors, HyprWorkspace};
use crate::datahodler::channel::{DualChannel, MReceiver, SSender};
use gtk::prelude::{ContainerExt, GridExt};
use gtk::traits::WidgetExt;
use gtk::traits::{BoxExt, ButtonExt, StyleContextExt};
use gtk::Widget;
use tracing::error;
use tracing::info;

#[derive(Clone)]
pub enum HyprOut {
    Parsed(ParsedEventType),
    AllWorkspaces(Vec<HyprWorkspace>, i32),
}

#[derive(Clone)]
pub enum HyprIn {
    NewClient,
}

#[derive(Default)]
pub struct HyprCurrentStatus {
    current_workspace_id: i32,
    current_monitor: Option<HyprMonitor>,
    current_window_title: String,
    current_window_class: String,
}

impl HyprCurrentStatus {
    pub fn get_current_monitor(&self) -> &HyprMonitor {
        self.current_monitor.as_ref().unwrap()
    }
}

enum MatchType {
    ID,
    Name,
}

#[derive(Clone)]
pub struct HyprWorkspaceWidget {
    name: String,
    id: i32,
    button: gtk::Button,
}

#[derive(Clone)]
pub struct HyprWidget {
    ww_map: Rc<RefCell<Vec<HyprWorkspaceWidget>>>,
    ws_box: gtk::Box,
    ws_title_button: gtk::Button,
    out_receiver: MReceiver<HyprOut>,
    in_sender: SSender<HyprIn>,
    holder: gtk::Box,
    current_status: Rc<RefCell<HyprCurrentStatus>>,
}

impl HyprWidget {
    pub fn new(
        in_sender: &SSender<HyprIn>,
        out_receiver: &MReceiver<HyprOut>,
        current_status: &Rc<RefCell<HyprCurrentStatus>>,
    ) -> Self {
        let holder = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        let ws_box = Self::create_workspace_container();
        let title_button = Self::create_active_window_button();

        holder.style_context().add_class("wm");

        holder.pack_start(&ws_box, false, false, 0);

        holder.pack_start(&title_button, false, false, 0);

        HyprWidget {
            ww_map: Default::default(),
            ws_box,
            ws_title_button: title_button,
            out_receiver: out_receiver.clone(),
            in_sender: in_sender.clone(),
            holder,
            current_status: current_status.clone(),
        }
    }

    async fn receive_out_events(&self) {
        let mut icon_loader = utils::gtk_icon_loader::GtkIconLoader::new();
        let mut receiver = self.out_receiver.clone();

        self.in_sender.send(HyprIn::NewClient).await.unwrap();

        loop {
            match receiver.recv().await {
                Ok(msg) => match msg {
                    HyprOut::Parsed(event) => match event {
                        ParsedEventType::WorkspaceChanged(ws) => self.on_workspace_changed(ws),
                        ParsedEventType::WorkspaceDeleted(ws) => self.on_workspace_deleted(ws),
                        ParsedEventType::WorkspaceAdded(ws) => self.on_workspace_added(ws),
                        ParsedEventType::WorkspaceMoved(ws, m) => self.on_workspace_moved(ws, m),
                        ParsedEventType::ActiveWindowChangedV1(class, title) => {
                            self.on_active_window_changed_v1(class, title)
                        }
                        ParsedEventType::ActiveMonitorChanged(monitor, ws) => {
                            self.on_active_monitor_changed(monitor, ws)
                        }
                        _ => {}
                    },
                    HyprOut::AllWorkspaces(vec, cursor) => self.init(vec, cursor),
                    _ => {}
                },
                Err(_) => {}
            }
        }
    }

    fn init(&self, wss: Vec<HyprWorkspace>, cursor: i32) {
        let ws = wss.iter().find(|e| e.id == cursor).unwrap();
        let current_monitor = ws.monitor.clone();

        self.current_status
            .borrow_mut()
            .current_monitor
            .replace(current_monitor);
        for ele in wss {
            self.add_workspace_directly(&ele);
        }

        self.holder.show_all();

        self.on_workspace_changed(cursor.to_string());
    }

    fn on_workspace_changed(&self, workspace: String) {
        {
            let current = self.current_status.borrow_mut();
            let ws_id = current.current_workspace_id;

            if let Some(ws) = self.find_ww(ws_id.to_string().as_str(), MatchType::ID) {
                let style = ws.button.style_context();
                if style.has_class("ws-focus") {
                    style.remove_class("ws-focus");
                }
            }
        }

        let ww = self.show_workspace(workspace.clone(), MatchType::Name);
        let style = ww.style_context();
        if !style.has_class("ws-focus") {
            style.add_class("ws-focus");
        }

        self.current_status.borrow_mut().current_workspace_id = workspace.parse::<i32>().unwrap();
    }

    fn on_workspace_deleted(&self, workspace: String) {
        self.hide_workspace(workspace, MatchType::Name)
    }

    fn on_workspace_added(&self, workspace: String) {
        self.show_workspace(workspace, MatchType::Name);
    }

    fn on_workspace_moved(&self, ws: String, monitor: String) {}

    fn on_active_window_changed_v1(&self, class: String, title: String) {}

    fn on_active_monitor_changed(&self, monitor: String, workspace: String) {}

    fn show_workspace(&self, workspace: String, match_type: MatchType) -> gtk::Button {
        let current_monitor = self.current_status.borrow().get_current_monitor().clone();

        let b = self.find_ww(workspace.as_str(), match_type);

        let but = match b {
            Some(but) => but.clone(),
            None => {
                let hypr_ws = HyprWorkspace {
                    id: workspace.parse().unwrap(),
                    name: workspace,
                    monitor: current_monitor.clone(),
                };
                let hww = Self::create_workspace_button(&hypr_ws);

                self.ww_map.borrow_mut().push(hww.clone());
                self.ws_box.pack_end(&hww.button, false, false, 0);

                hww
            }
        };

        Self::reorder_workspaces(&self.ws_box);

        but.button.show();

        but.button
    }

    fn add_workspace_directly(&self, workspace: &HyprWorkspace) {
        let ws = Self::create_workspace_button(workspace);
        self.ws_box.pack_end(&ws.button, false, false, 0);
        self.ww_map.borrow_mut().push(ws);
    }

    fn hide_workspace(&self, workspace: String, match_type: MatchType) {
        let b = self.find_ww(workspace.as_str(), match_type);

        match b {
            Some(but) => {
                but.button.hide();
                but.button.style_context().remove_class("ws-focus");
            }
            None => {}
        };
    }

    fn reorder_workspaces(workspaces: &gtk::Box) {
        let mut children = workspaces.children();
        children.sort_by(|a, b| a.widget_name().cmp(&b.widget_name()));

        children
            .iter()
            .rev()
            .enumerate()
            .for_each(|(i, b)| workspaces.reorder_child(b, i as i32));
    }

    fn create_workspace_button(ws: &HyprWorkspace) -> HyprWorkspaceWidget {
        let label = ws.get_bar_name();

        let workspace_button = gtk::Button::builder()
            .label(label)
            .name(&ws.id.to_string())
            .build();

        workspace_button.style_context().add_class("ws");

        workspace_button.connect_clicked(move |but| {
            let id = but.widget_name();

            hyprctl_switch_to_workspace(id.as_str())
        });

        HyprWorkspaceWidget {
            name: ws.name.clone(),
            id: ws.id.clone(),
            button: workspace_button,
        }
    }

    fn create_workspace_container() -> gtk::Box {
        let ws_container = gtk::Box::builder().build();
        ws_container.style_context().add_class("wss");

        ws_container
    }

    fn create_active_window_button() -> gtk::Button {
        let active_window = gtk::Button::builder().build();
        active_window.style_context().add_class("wm-title");

        active_window
    }

    fn find_ww(&self, name: &str, match_type: MatchType) -> Option<HyprWorkspaceWidget> {
        self.ww_map
            .borrow()
            .iter()
            .find(|e| match match_type {
                MatchType::ID => e.id.to_string().as_str() == name,
                MatchType::Name => e.name.as_str() == name,
            })
            .map(|e| e.clone())
    }
}

pub struct HyprBlock {
    dualchannel: DualChannel<HyprOut, HyprIn>,
    current_status: Rc<RefCell<HyprCurrentStatus>>,
}

impl HyprBlock {
    pub fn new() -> Self {
        let current_status = Rc::new(RefCell::default());

        Self {
            dualchannel: DualChannel::new(30),
            current_status,
        }
    }

    fn new_client() -> (Vec<HyprWorkspace>, i32) {
        let workspaces = hyprclients::get_workspaces().unwrap();
        let active_workspace = hyprclients::get_active_workspace().unwrap();

        (workspaces, active_workspace.id)
    }

    fn handle_input_msg(msg: HyprIn) {
        match msg {
            HyprIn::NewClient => {}
        }
    }
}

impl Block for HyprBlock {
    type Out = HyprOut;
    type In = HyprIn;

    fn run(&mut self) -> anyhow::Result<()> {
        if let Ok(ins) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
            let sender = self.dualchannel.get_out_sender();

            let socket = format!("/tmp/hypr/{}/.socket2.sock", ins);
            let socket_path = Path::new(socket.as_str());

            info!("Listening on: {:?}", socket_path);
            let regexes = hyprevents::get_event_regex();
            let socket_address = gio::UnixSocketAddress::new(socket_path);

            MainContext::ref_thread_default().spawn_local(async move {
                loop {
                    let client = SocketClient::new();
                    let connection_result = client.connect(
                        &gio::SocketConnectable::from(socket_address.clone()),
                        None::<&gio::Cancellable>,
                    );

                    if let Ok(conn) = connection_result {
                        let arw = conn.into_async_read_write().unwrap();
                        let dis = DataInputStream::new(arw.input_stream());

                        loop {
                            let future = dis.read_line_utf8_future(Priority::DEFAULT);
                            match future.await {
                                Ok(Some(line)) => {
                                    let event =
                                        hyprevents::convert_line_to_event(&regexes, line.as_str());
                                    sender.send(HyprOut::Parsed(event)).unwrap();
                                }
                                Ok(None) => {
                                    error!("receive events none.");
                                    break;
                                }
                                Err(err) => {
                                    error!("receive events error: {:?}", err);
                                    break;
                                }
                            }
                        }
                    }
                }
            });

            let in_receiver = self.dualchannel.get_in_recevier();
            let sender = self.dualchannel.get_out_sender();
            MainContext::ref_thread_default().spawn_local(async move {
                loop {
                    match in_receiver.recv().await {
                        Ok(msg) => match msg {
                            HyprIn::NewClient => {
                                let all = Self::new_client();
                                sender.send(HyprOut::AllWorkspaces(all.0, all.1)).unwrap();
                            }
                        },
                        Err(_) => todo!(),
                    }
                }
            });

            Ok(())
        } else {
            Err(anyhow!("Is hyprland is running?"))
        }
    }

    fn get_channel(&self) -> (&SSender<Self::In>, &MReceiver<Self::Out>) {
        todo!()
    }

    fn widget(&self) -> Widget {
        let in_sender = self.dualchannel.get_in_sender();
        let out_receiver = self.dualchannel.get_out_receiver();

        let hypr_widget = HyprWidget::new(&in_sender, &out_receiver, &self.current_status);

        let _hypr_widget = hypr_widget.clone();
        MainContext::ref_thread_default().spawn_local(async move {
            _hypr_widget.receive_out_events().await;
        });

        hypr_widget.holder.upcast()
    }
}

fn hyprctl_switch_to_workspace(id: &str) {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("workspace")
        .arg(id)
        .spawn()
        .unwrap();
}
