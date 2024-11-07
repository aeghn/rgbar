pub mod backend;

use backend::WaylandWidget;
use chin_tools::wayland::{into_wl_event, WLCompositor, WLEvent, WLWorkspace};
use chin_tools::wrapper::anyhow::AResult;
use gdk::glib::Cast;
use gtk::Widget;

use crate::datahodler::channel::{DualChannel, MReceiver, SSender};
use crate::statusbar::WidgetShareInfo;
use gio::{DataInputStream, SocketClient};
use glib::{MainContext, Priority};

use super::Block;
use std::cell::RefCell;
use std::path::Path;
use std::process::Command;

#[derive(Clone)]
pub enum OutEvent {
    WLEvent(WLEvent),
    AllWorkspaces(Vec<WLWorkspace>),
}

#[derive(Clone)]
pub enum InEvent {
    NewBar,
}

pub struct WaylandBlock {
    dualchannel: DualChannel<OutEvent, InEvent>,
}

impl WaylandBlock {
    pub fn new() -> Self {
        Self {
            dualchannel: DualChannel::new(30),
        }
    }

    fn new_client() -> AResult<Vec<WLWorkspace>> {
        let com = WLCompositor::current()?;
        let workspaces = com.get_all_workspaces()?;

        Ok(workspaces)
    }
}

impl Block for WaylandBlock {
    type Out = OutEvent;
    type In = InEvent;

    fn run(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();

        std::thread::spawn(move || {
            let mut all_windows = chin_tools::wayland::niri::model::Window::get_all()
                .unwrap()
                .into_iter()
                .map(|e| (e.id, e))
                .collect();
            let mut all_workspace = chin_tools::wayland::niri::model::Workspace::get_all()
                .unwrap()
                .into_iter()
                .map(|e| (e.id, e))
                .collect();

            chin_tools::wayland::niri::event_stream::handle_event_stream(|event| {
                let events = into_wl_event(event, &mut all_workspace, &mut all_windows);
                if let Some(events) = events {
                    for ele in events {
                        let _ = sender.send(OutEvent::WLEvent(ele));
                    }
                }
            })
        });

        let in_receiver = self.dualchannel.get_in_receiver();
        let sender = self.dualchannel.get_out_sender();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match in_receiver.recv().await {
                    Ok(msg) => match msg {
                        InEvent::NewBar => {
                            let all = Self::new_client().unwrap();
                            sender.send(OutEvent::AllWorkspaces(all)).unwrap();
                        }
                    },
                    Err(_) => todo!(),
                }
            }
        });

        Ok(())
    }

    fn widget(&self, share_info: &WidgetShareInfo) -> Widget {
        let in_sender = self.dualchannel.get_in_sender();
        let out_receiver = self.dualchannel.get_out_receiver();

        let wayland_widget = WaylandWidget::new(&in_sender, &out_receiver, share_info).unwrap();

        {
            let wayland_widget = wayland_widget.clone();
            MainContext::ref_thread_default().spawn_local(async move {
                wayland_widget.receive_out_events().await;
            });
        }

        wayland_widget.holder.upcast()
    }
}
