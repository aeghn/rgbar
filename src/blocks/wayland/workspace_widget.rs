use std::collections::HashMap;

use chin_tools::wayland::{WLWorkspace, WLWorkspaceBehaiver, WLWorkspaceId};
use chin_tools::wrapper::anyhow::AResult;
pub use gtk::traits::{BoxExt, LabelExt, StyleContextExt, WidgetExt};

#[derive(Debug, PartialEq)]
pub struct WorkspaceWidget {
    workspace: WLWorkspace,
}

impl WorkspaceWidget {
    pub fn new(workspace: WLWorkspace) -> WorkspaceWidget {
        WorkspaceWidget { workspace }
    }
}

#[derive(Debug)]
pub struct WorkspaceContainer {
    pub workspace_widget_map: HashMap<WLWorkspaceId, WorkspaceWidget>,
    pub holder: gtk::Box,
    indicator: gtk::Label,
    output_name: String,
    current_workspace_id: Option<WLWorkspaceId>,
}

impl WorkspaceContainer {
    pub fn new(output_name: String) -> AResult<Self> {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        let indicator = gtk::Label::builder().build();
        indicator.style_context().add_class("ws");

        holder.style_context().add_class("wss");
        holder.pack_start(&indicator, false, true, 0);

        Ok(Self {
            workspace_widget_map: Default::default(),
            holder,
            output_name,
            current_workspace_id: Default::default(),
            indicator,
        })
    }

    pub fn on_workspace_overwrite(&mut self, workspace: &WLWorkspace) {
        if workspace.is_focused {
            self.current_workspace_id.replace(workspace.id);
        }
        self.workspace_widget_map
            .insert(workspace.get_id(), WorkspaceWidget::new(workspace.clone()));

        self.update_view();
    }

    pub fn on_workspace_delete(&mut self, id: &WLWorkspaceId) {
        self.workspace_widget_map.remove(&id);
        self.update_view();
    }

    pub fn update_view(&self) {
        let indicator = self
            .current_workspace_id
            .and_then(|e| {
                self.workspace_widget_map.get(&e).and_then(|e| {
                    if e.workspace.output.as_ref() == Some(&self.output_name) {
                        Some(e)
                    } else {
                        None
                    }
                })
            })
            .map_or("?".to_owned(), |e| e.workspace.get_name());
        self.indicator.set_label(
            format!(
                "{}/{}",
                indicator,
                self.workspace_widget_map
                    .iter()
                    .filter(|(_, ws)| ws.workspace.output.as_ref() == Some(&self.output_name))
                    .count()
            )
            .as_str(),
        );
    }
}
