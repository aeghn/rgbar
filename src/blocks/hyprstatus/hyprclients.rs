
use std::process::Command;




pub struct HyprWorkspace {
    pub id: i64,
    pub monitor: i32,
    pub name: String,
}

pub struct HyprWindowResult {
    pub class: String,
    pub title: String,
    pub address: String,
    pub mapped: bool,
    pub hidden: bool,
    pub pid: i64,
    pub xwayland: bool,
    pub workspace: HyprWorkspace,
}

pub fn get_clients() -> Result<Vec<HyprWindowResult>, String> {
    let output = Command::new("hyprctl")
        .arg("clients")
        .arg("-j")
        .output()
        .unwrap();

    let mut vec: Vec<HyprWindowResult> = vec![];

    let out = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(out.as_str()).unwrap();

    if let Some(array) = json.as_array() {
        for e in array {
            let class = e.get("class").unwrap().as_str().unwrap();
            let monitor = e.get("monitor").unwrap().as_i64().unwrap();
            if monitor == -1 {
                continue;
            }

            vec.push(HyprWindowResult {
                class: class.to_string(),
                title: e.get("title").unwrap().as_str().unwrap().to_string(),
                address: e.get("address").unwrap().as_str().unwrap().to_string(),
                mapped: e.get("mapped").unwrap().as_bool().unwrap(),
                hidden: e.get("hidden").unwrap().as_bool().unwrap(),
                pid: e.get("pid").unwrap().as_i64().unwrap(),
                xwayland: e.get("xwayland").unwrap().as_bool().unwrap(),
                workspace: HyprWorkspace {
                    id: e
                        .get("workspace")
                        .unwrap()
                        .get("id")
                        .unwrap()
                        .as_i64()
                        .unwrap(),
                    monitor: monitor as i32,
                    name: e
                        .get("workspace")
                        .unwrap()
                        .get("name")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                },
            })
        }
    }
    Ok(vec)
}

pub fn get_active_window() -> Option<String> {
    let output = Command::new("hyprctl")
        .arg("activewindow")
        .arg("-j")
        .output()
        .unwrap();

    let _vec: Vec<HyprWindowResult> = vec![];

    let out = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(out.as_str()).unwrap();

    if let Some(obj) = json.as_object() {
        if let Some(address) = obj.get("address") {
            Some(address.as_str().unwrap().to_string())
        } else {
            None
        }
    } else {
        None
    }
}
