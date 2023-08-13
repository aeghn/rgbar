use std::fs;

static IDEAPAD_ACPI: &str = "/sys/bus/platform/drivers/ideapad_acpi/VPC2004:00";
static CONSERVATION_MODE: &str = "conservation_mode";
// static FN_LOCK : &str =  format!("{}/{}", IDEAPAD_ACPI, "fn_lock").as_str();
// static CAMERA_POWER : &str =  format!("{}/{}", IDEAPAD_ACPI, "camera_power").as_str();
// static FAN_MODE : &str =  format!("{}/{}", IDEAPAD_ACPI, "fan_mode").as_str();

pub fn is_conservation_mode() -> bool {
    let str = fs::read_to_string(format!("{}/{}", IDEAPAD_ACPI, CONSERVATION_MODE));
    str.unwrap().starts_with("1")
}
