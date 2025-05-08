pub mod window_widget;
pub mod workspace_widget;

use chin_tools::wayland::{into_wl_event, WLEvent};


use window_widget::{WindowContainer, WindowContainerManager};
use workspace_widget::WorkspaceContainer;

use crate::datahodler::channel::DualChannel;
use crate::window::WidgetShareInfo;

use super::Block;

use chin_tools::utils::id_util;
use crate::prelude::*;

use tracing::error;

#[derive(Clone)]
pub enum OutEvent {
    WLEvent(WLEvent),
}

#[derive(Clone)]
pub enum InEvent {}

pub struct WaylandBlock {
    dualchannel: DualChannel<OutEvent, InEvent>,
}

impl Block for WaylandBlock {
    type Out = OutEvent;
    type In = InEvent;

    fn run(&mut self) -> AResult<()> {
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
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = in_receiver.recv().await {
                    match msg {}
                }
            }
        });

        Ok(())
    }

    fn widget(&self, share_info: &WidgetShareInfo) -> Widget {
        let output_name = share_info
            .plug_name
            .as_ref()
            .map_or_else(|| id_util::generate_uuid(), |s| s.to_owned());

        let mut workspace_container = WorkspaceContainer::new(output_name.clone())
            .unwrap()
            .init()
            .unwrap();

        let mut window_container = WindowContainerManager::new()
            .unwrap()
            .init(workspace_container.get_workspace_ids())
            .unwrap();

        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        holder.style_context().add_class("wm");

        holder.pack_start(&workspace_container.holder, false, false, 0);

        holder.pack_start(&window_container.stack, false, false, 0);

        let mut receiver = self.dualchannel.get_out_receiver().clone();

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match receiver.recv().await {
                    Ok(msg) => match msg {
                        OutEvent::WLEvent(event) => match event {
                            WLEvent::WorkspaceFocused(ws) => {
                                if ws
                                    .get_output_name()
                                    .map_or(true, |name| name != output_name)
                                {
                                    continue;
                                }
                                window_container.on_workspace_change(ws.get_id());
                                workspace_container.on_workspace_focused(&ws)
                            }
                            WLEvent::WorkspaceDeleted(ws) => {
                                if ws
                                    .get_output_name()
                                    .map_or(true, |name| name != output_name)
                                {
                                    continue;
                                }
                                window_container.on_workspace_delete(ws.get_id());
                                workspace_container.on_workspace_delete(&ws);
                            }
                            WLEvent::WorkspaceAdded(ws) => {
                                if ws
                                    .get_output_name()
                                    .map_or(true, |name| name != output_name)
                                {
                                    continue;
                                }
                                window_container
                                    .on_workspace_overwrite(WindowContainer::new(ws.get_id()));

                                workspace_container.on_workspace_added(&ws);
                            }
                            WLEvent::WorkspaceChanged(ws) => {
                                if ws
                                    .get_output_name()
                                    .map_or(true, |name| name != output_name)
                                {
                                    continue;
                                }
                                window_container.on_workspace_change(ws.get_id());
                                workspace_container.on_workspace_changed(&ws);
                            }
                            WLEvent::WindowFocused(window) => {
                                window_container.on_window_change_focus(window)
                            }
                            WLEvent::MonitorFocused(output) => {
                                workspace_container.on_active_monitor_changed(&output)
                            }
                            WLEvent::WindowDeleted(window) => {
                                window_container.on_window_delete(window)
                            }
                            WLEvent::WindowOverwrite(window) => {
                                window_container.on_window_overwrite(window)
                            }
                        },
                    },
                    Err(err) => {
                        error!("unable to receive message: {}", err)
                    }
                }
            }
        });

        holder.clone().upcast()
    }
}

impl WaylandBlock {
    pub fn new() -> Self {
        Self {
            dualchannel: DualChannel::new(30),
        }
    }
}
