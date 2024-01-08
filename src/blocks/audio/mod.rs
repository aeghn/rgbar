pub enum VolumeOperateMsg {
    Mute,
    SetVolume(u8),
    Increase(u8),
    Decrease(u8),
    GetVolume,
}

pub enum VolumeEchoMsg {
    Muted(bool),
    Volume(u8),
    Earphone(bool)
}

trait Speaker {
    fn do_operation(&self, msg: VolumeOperateMsg);
}

trait Mic {

}
