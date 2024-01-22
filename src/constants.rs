pub const ERR_GET_DISPLAY: &str =
    "[ERROR] Couldn't find a valid display, is your compositor doing alright?";
pub const ERR_GET_MONITOR: &str = "[ERROR] Couldn't find a valid monitor.";
pub const ERR_CUSTOM_DRAW: &str =
    "[ERROR] Failed drawing Hybrid using custom color sources, which is needed for transparency!";

#[derive(Clone)]
pub enum TriBool {
    True,
    False,
    Unknown
}