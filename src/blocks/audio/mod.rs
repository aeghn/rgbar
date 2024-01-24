use libpulse_binding::volume::ChannelVolumes;

use crate::datahodler::channel::DualChannel;

#[derive(Clone)]
pub enum PulseBM {
    Mute,
    SetVolume(u8),
    Increase(u8),
    Decrease(u8),
    GetVolume,
}

#[derive(Clone)]
pub enum PulseWM {
    Muted(bool),
    Volume(u8),
    Earphone(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    Sink,
    Source,
}

pub struct PulseBlock {
    dualchannel: DualChannel<PulseWM, PulseBM>,
    name: Option<String>,
    description: Option<String>,
    active_port: Option<String>,
    form_factor: Option<String>,
    volume: Option<ChannelVolumes>,
    volume_avg: u32,
    muted: bool,
}

impl PulseBlock {
    pub fn new() -> Self {
        PulseBlock {
            dualchannel: DualChannel::new(30),
            name: Default::default(),
            description: Default::default(),
            active_port: Default::default(),
            form_factor: Default::default(),
            volume: Default::default(),
            volume_avg: Default::default(),
            muted: Default::default(),
        }
    }
}
