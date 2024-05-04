pub mod hyprclients;
pub mod hyprevents;

use crate::statusbar::WidgetShareInfo;
use crate::utils::{self};
use anyhow::anyhow;

use gio::prelude::{DataInputStreamExtManual, IOStreamExtManual};
use gio::traits::SocketClientExt;
use gio::{DataInputStream, SocketClient};
use glib::{Cast, MainContext, Priority};
use hyprevents::ParsedEventType;

use std::cell::RefCell;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

use self::hyprclients::HyprMonitor;

use super::Block;
use crate::blocks::hyprstatus::hyprclients::HyprWorkspace;
use crate::datahodler::channel::{DualChannel, MReceiver, SSender};
use crate::utils::gtkiconloader::GtkIconLoader;
use gtk::prelude::{ContainerExt, ImageExt, LabelExt};
use gtk::traits::WidgetExt;
use gtk::traits::{BoxExt, ButtonExt, StyleContextExt};
use gtk::{Label, Widget};
use tracing::error;
use tracing::info;

#[derive(Clone)]
pub enum HyprOut {
    Parsed(ParsedEventType),
    AllWorkspaces(Vec<HyprWorkspace>, String, String, String),
}

#[derive(Clone)]
pub enum HyprIn {
    NewClient,
}

#[derive(Default, Debug)]
pub struct HyprCurrentStatus {
    current_workspace_name: String,
    current_monitor: Option<HyprMonitor>,
    current_window_title: String,
    current_window_class: String,
}

impl HyprCurrentStatus {
    pub fn get_current_monitor(&self) -> Option<HyprMonitor> {
        self.current_monitor.clone()
    }
}

#[derive(Debug)]
enum MatchType {
    Name,
}

#[derive(Clone, Debug)]
pub struct HyprWorkspaceWidget {
    name: String,
    button: gtk::Button,
}

#[derive(Clone)]
pub struct HyprWidget {
    ww_vec: Rc<RefCell<Vec<HyprWorkspaceWidget>>>,
    ws_box: gtk::Box,
    cw_title: (gtk::Image, Label),
    out_receiver: MReceiver<HyprOut>,
    in_sender: SSender<HyprIn>,
    holder: gtk::Box,
    current_status: Rc<RefCell<HyprCurrentStatus>>,
    pub icon_loader: GtkIconLoader,
}

impl HyprWidget {
    pub fn new(
        in_sender: &SSender<HyprIn>,
        out_receiver: &MReceiver<HyprOut>,
        _share_info: &WidgetShareInfo,
    ) -> Self {
        let holder = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        let ws_box = Self::create_workspace_container();
        let title_container = Self::create_active_window_container();

        holder.style_context().add_class("wm");

        holder.pack_start(&ws_box, false, false, 0);

        holder.pack_start(&title_container.0, false, false, 0);
        holder.pack_start(&title_container.1, false, false, 0);

        let icon_loader = utils::gtkiconloader::GtkIconLoader::new();

        HyprWidget {
            ww_vec: Default::default(),
            ws_box,
            cw_title: title_container,
            out_receiver: out_receiver.clone(),
            in_sender: in_sender.clone(),
            holder,
            current_status: Default::default(),
            icon_loader,
        }
    }

    async fn receive_out_events(&self) {
        let mut receiver = self.out_receiver.clone();

        self.in_sender.send(HyprIn::NewClient).await.unwrap();

        loop {
            match receiver.recv().await {
                Ok(msg) => match msg {
                    HyprOut::Parsed(event) => match event {
                        ParsedEventType::WorkspaceChanged(ws) => self.on_workspace_changed(&ws),
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
                    HyprOut::AllWorkspaces(vec, cursor, class, title) => {
                        self.init(vec, cursor, class, title)
                    }
                },
                Err(err) => {
                    error!("unable receive messsage: {}", err)
                }
            }
        }
    }

    fn init(&self, wss: Vec<HyprWorkspace>, wname: String, class: String, title: String) {
        let ws = wss.iter().find(|e| e.name == wname).unwrap();
        let current_monitor = ws.monitor.clone();

        let current_status = HyprCurrentStatus {
            current_workspace_name: wname.clone(),
            current_monitor: Some(current_monitor),
            current_window_title: title.clone(),
            current_window_class: class.clone(),
        };

        self.current_status.replace(current_status);
        for ele in wss {
            self.add_workspace_directly(&ele);
        }

        self.on_active_window_changed_v1(class, title);

        self.holder.show_all();

        self.on_workspace_changed(&wname);
    }

    fn on_workspace_changed(&self, workspace: &String) {
        {
            let current = self.current_status.borrow_mut();
            let ws_name = current.current_workspace_name.as_str();

            if let Some(ws) = self.find_ww(ws_name, MatchType::Name) {
                let style = ws.button.style_context();
                if style.has_class("ws-focus") {
                    style.remove_class("ws-focus");
                }
            }
        }

        let ww = self.show_workspace(workspace.clone(), MatchType::Name);
        if let Some(ww) = ww {
            let style = ww.style_context();
            if !style.has_class("ws-focus") {
                style.add_class("ws-focus");
            }
        }

        self.current_status.borrow_mut().current_workspace_name = workspace.clone();
    }

    fn on_workspace_deleted(&self, workspace: String) {
        self.hide_workspace(workspace, MatchType::Name)
    }

    fn on_workspace_added(&self, workspace: String) {
        self.show_workspace(workspace, MatchType::Name);
    }

    fn on_workspace_moved(&self, _ws: String, _monitor: String) {}

    fn on_active_window_changed_v1(&self, class: String, title: String) {
        let mut current_status = self.current_status.borrow_mut();
        if current_status.current_window_title != title {
            current_status.current_window_title = title.clone();
        }

        let visiable = self.cw_title.1.is_visible();
        let is_empty = current_status.current_window_title.is_empty();

        if is_empty && visiable {
            self.cw_title.1.hide();
            self.cw_title.0.hide();
        }
        self.cw_title.1.set_label(title.as_str());

        if let Some(img) = self.icon_loader.load_from_name(&class) {
            self.cw_title.0.set_from_pixbuf(Some(&img));
            self.cw_title.0.show();
        } else {
            self.cw_title.0.hide();
        }

        if !is_empty && !visiable {
            self.cw_title.1.show();
        }
    }

    fn on_active_monitor_changed(&self, monitor: String, workspace: String) {
        self.current_status
            .borrow_mut()
            .current_monitor
            .replace(HyprMonitor {
                id: None,
                name: monitor,
            });
        self.on_workspace_changed(&workspace);
    }

    fn show_workspace(&self, workspace_name: String, match_type: MatchType) -> Option<gtk::Button> {
        let current_monitor = self.current_status.borrow().get_current_monitor();

        let b = self.find_ww(workspace_name.as_str(), match_type);

        if let Some(mon) = current_monitor {
            let but = match b {
                Some(but) => but.clone(),
                None => {
                    let hypr_ws = HyprWorkspace {
                        id: None,
                        name: workspace_name,
                        monitor: mon.clone(),
                    };
                    let hww = Self::create_workspace_button(&hypr_ws);

                    self.ww_vec.borrow_mut().push(hww.clone());
                    self.ws_box.pack_end(&hww.button, false, false, 0);

                    hww
                }
            };

            Self::reorder_workspaces(&self.ws_box);

            but.button.show();

            Some(but.button)
        } else {
            None
        }
    }

    fn add_workspace_directly(&self, workspace: &HyprWorkspace) {
        let _ws = match self.find_ww(workspace.name.as_str(), MatchType::Name) {
            None => {
                let widget = Self::create_workspace_button(workspace);
                self.ww_vec.borrow_mut().push(widget.clone());
                self.ws_box.pack_end(&widget.button, false, false, 0);
                widget
            }
            Some(ws) => ws,
        };
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
        children.sort_by(|a, b| {
            let ai = isize::from_str_radix(a.widget_name().as_str(), 10);
            let bi = isize::from_str_radix(b.widget_name().as_str(), 10);

            if let (Ok(ai), Ok(bi)) = (ai, bi) {
                isize::cmp(&ai, &bi)
            } else {
                a.widget_name().cmp(&b.widget_name())
            }
        });

        children
            .iter()
            .rev()
            .enumerate()
            .for_each(|(i, b)| workspaces.reorder_child(b, i as i32));
    }

    fn create_workspace_button(ws: &HyprWorkspace) -> HyprWorkspaceWidget {
        let label = ws.get_bar_name();

        let workspace_button = gtk::Button::builder().label(label).name(&ws.name).build();

        workspace_button.style_context().add_class("ws");

        workspace_button.connect_clicked(move |but| {
            let id = but.widget_name();

            hyprctl_switch_to_workspace(id.as_str())
        });

        HyprWorkspaceWidget {
            name: ws.name.clone(),
            button: workspace_button,
        }
    }

    fn create_workspace_container() -> gtk::Box {
        let ws_container = gtk::Box::builder().build();
        ws_container.style_context().add_class("wss");

        ws_container
    }

    fn create_active_window_container() -> (gtk::Image, gtk::Label) {
        let image = gtk::Image::builder().build();
        image.style_context().add_class("wm-cw-icon");
        let label = gtk::Label::builder().build();
        label.style_context().add_class("wm-cw-title");

        label.set_single_line_mode(true);
        label.set_ellipsize(gdk::pango::EllipsizeMode::End);
        label.set_lines(1);
        label.set_line_wrap(true);
        label.set_line_wrap_mode(gdk::pango::WrapMode::Char);

        (image, label)
    }

    fn find_ww(&self, name: &str, match_type: MatchType) -> Option<HyprWorkspaceWidget> {
        let fount = self
            .ww_vec
            .borrow()
            .iter()
            .find(|e| match match_type {
                MatchType::Name => e.name.as_str() == name,
            })
            .map(|e| e.clone());
        fount
    }
}

pub struct HyprBlock {
    dualchannel: DualChannel<HyprOut, HyprIn>,
}

impl HyprBlock {
    pub fn new() -> Self {
        Self {
            dualchannel: DualChannel::new(30),
        }
    }

    fn new_client() -> (Vec<HyprWorkspace>, String, String, String) {
        let workspaces = hyprclients::get_workspaces().unwrap();
        let active_workspace = hyprclients::get_active_workspace().unwrap();
        let client = hyprclients::get_active_client();
        let title = client
            .as_ref()
            .map(|e| e.title.clone())
            .unwrap_or("".to_string());
        let class = client
            .as_ref()
            .map(|e| e.class.clone())
            .unwrap_or("".to_string());

        (workspaces, active_workspace.name, class, title)
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
                                sender
                                    .send(HyprOut::AllWorkspaces(all.0, all.1, all.2, all.3))
                                    .unwrap();
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

    fn widget(&self, share_info: &WidgetShareInfo) -> Widget {
        let in_sender = self.dualchannel.get_in_sender();
        let out_receiver = self.dualchannel.get_out_receiver();

        let hypr_widget = HyprWidget::new(&in_sender, &out_receiver, share_info);

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
