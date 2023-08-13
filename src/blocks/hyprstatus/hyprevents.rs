use crate::blocks::hyprstatus::hyprevents::ParsedEventType::{
    ActiveWindowChangedV1, Unknown, WorkspaceAdded, WorkspaceChanged, WorkspaceDeleted,
    WorkspaceMoved,
};
use regex::Regex;

type MonitorType = String;
type WorkspaceType = String;
type TitleType = String;
type ClassType = String;
type AddressType = String;

#[derive(PartialEq, Eq, Hash)]
pub enum ParsedEventType {
    WorkspaceChanged(WorkspaceType),
    WorkspaceDeleted(WorkspaceType),
    WorkspaceAdded(WorkspaceType),
    WorkspaceMoved(WorkspaceType, MonitorType),
    ActiveWindowChangedV1(ClassType, TitleType),
    ActiveWindowChangedV2(AddressType),
    ActiveMonitorChanged(MonitorType, WorkspaceType),
    FullscreenStateChanged,
    MonitorAdded,
    MonitorRemoved,
    WindowOpened,
    WindowClosed,
    WindowMoved,
    LayoutChanged,
    SubMapChanged,
    LayerOpened,
    LayerClosed,
    FloatStateChanged,
    UrgentStateChanged,
    Minimize,
    WindowTitleChanged(AddressType),
    Screencast,
    Unknown,
}

pub fn get_event_regex() -> Vec<Regex> {
    vec![
        r"\bworkspace>>(?P<workspace>.*)",
        r"destroyworkspace>>(?P<workspace>.*)",
        r"createworkspace>>(?P<workspace>.*)",
        r"moveworkspace>>(?P<workspace>.*),(?P<monitor>.*)",
        r"focusedmon>>(?P<monitor>.*),(?P<workspace>.*)",
        r"activewindow>>(?P<class>.*)?,(?P<title>.*)",
        r"activewindowv2>>(?P<address>.*)",
        r"fullscreen>>(?P<state>0|1)",
        r"monitorremoved>>(?P<monitor>.*)",
        r"monitoradded>>(?P<monitor>.*)",
        r"openwindow>>(?P<address>.*),(?P<workspace>.*),(?P<class>.*),(?P<title>.*)",
        r"closewindow>>(?P<address>.*)",
        r"movewindow>>(?P<address>.*),(?P<workspace>.*)",
        r"activelayout>>(?P<keyboard>.*)(?P<layout>.*)",
        r"submap>>(?P<submap>.*)",
        r"openlayer>>(?P<namespace>.*)",
        r"closelayer>>(?P<namespace>.*)",
        r"changefloatingmode>>(?P<address>.*),(?P<floatstate>[0-1])",
        r"minimize>>(?P<address>.*),(?P<state>[0-1])",
        r"screencast>>(?P<state>[0-1]),(?P<owner>[0-1])",
        r"urgent>>(?P<address>.*)",
        r"windowtitle>>(?P<address>.*)",
        r"(?P<Event>^[^>]*)",
    ]
    .iter()
    .map(|s| Regex::new(s).unwrap())
    .collect()
}

pub fn convert_line_to_event(regex_set: &Vec<Regex>, line: &str) -> ParsedEventType {
    for (id, regex) in regex_set.iter().enumerate() {
        match regex.captures(line) {
            None => {}
            Some(caps) => match id {
                0 => return WorkspaceChanged(caps["workspace"].to_string()),
                1 => return WorkspaceDeleted(caps["workspace"].to_string()),
                2 => return WorkspaceAdded(caps["workspace"].to_string()),
                3 => {
                    return WorkspaceMoved(
                        caps["workspace"].to_string(),
                        caps["monitor"].to_string(),
                    )
                }
                5 => {
                    return ActiveWindowChangedV1(
                        caps["class"].to_string(),
                        caps["title"].to_string(),
                    )
                }
                _ => {
                    return Unknown;
                }
            },
        }
    }
    Unknown
}
