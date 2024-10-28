pub fn second_to_human(secs: u32) -> String {
    let minute = secs % 3600 / 60;
    let hour = secs / 3600;

    if hour > 0 {
        format!("{}:{}", hour, minute)
    } else {
        format!("{}", minute)
    }
}
