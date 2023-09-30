pub enum VolumeOperateMsg {
    Mute,
    SetVolume(u8),
    Increase(u8),
    Decrease(u8),
    GetVolume,
}

trait Speaker {
    fn is_mute() -> bool;

}

trait Mic {

}

#[cfg(test)]
mod tests {
    use pulsectl::controllers::{DeviceControl, SinkController};

    #[test]
    fn pulse_test() {
        let mut handler = SinkController::create().unwrap();

        let devices = handler
            .list_devices()
            .expect("Could not get list of playback devices.");

        println!("Playback Devices: ");
        for dev in devices.clone() {
            println!(
                "[{}] {}, Volume: {}",
                dev.index,
                dev.description.as_ref().unwrap(),
                dev.volume.print()
            );
        }
    }
}