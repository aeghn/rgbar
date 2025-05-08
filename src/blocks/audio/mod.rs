#[allow(dead_code)]
pub mod pulse;


use crate::prelude::*;
use crate::util::gtk_icon_loader::load_fixed_status_surface;

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, SystemTime},
};

use crate::{
    datahodler::channel::{DualChannel, MSender},
    util::gtk_icon_loader::{self, StatusName},
};

use self::pulse::Device;

use super::Block;

#[derive(Clone)]
#[allow(dead_code)]
pub enum PulseBM {
    ToggleMute,
    SetVolume(u32),
    Increase(u32),
    Decrease(u32),
    GetVolume,
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum PulseWM {
    Muted(bool),
    Volume(u32),
    Earphone(bool),
    Full {
        muted: bool,
        vol: u32,
        device_type: DeviceType,
    }, // Muted, volume, earphone
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum DeviceType {
    Headset,
    Headphone,
    HandsFree,
    Portable,
    Default,
}

#[allow(dead_code)]
trait SoundDevice {
    fn volume(&self) -> u32;
    fn muted(&self) -> bool;
    fn output_name(&self) -> String;
    fn output_description(&self) -> Option<String>;
    fn active_port(&self) -> Option<String>;
    fn form_factor(&self) -> Option<&str>;

    async fn get_info(&mut self) -> AResult<()>;
    fn set_volume(&self, step: i32, max_vol: Option<u32>) -> AResult<()>;
    async fn toggle(&self) -> AResult<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum DeviceKind {
    Sink,
    Source,
}

pub struct PulseBlock {
    dualchannel: DualChannel<PulseWM, PulseBM>,
    default_sink: Rc<RefCell<Device>>,
}

impl PulseBlock {
    pub fn new() -> Self {
        let dualchannel: DualChannel<PulseWM, PulseBM> = DualChannel::new(32);
        let default_sink = Rc::new(RefCell::new(
            Device::new(
                crate::blocks::audio::DeviceKind::Sink,
                None,
                dualchannel.get_in_sender(),
            )
            .unwrap(),
        ));

        PulseBlock {
            dualchannel,
            default_sink,
        }
    }

    fn is_headphone(device: &Device) -> DeviceType {
        let active_port = device.active_port();

        macro_rules! check {
            ($key:expr, $res:tt) => {
                if let Some($key) = device.form_factor() {
                    return DeviceType::$res;
                }
                if active_port.as_ref().is_some_and(|p| p.contains($key)) {
                    return DeviceType::$res;
                }
            };
        }

        check!("headset", Headset);
        check!("headphone", Headphone);
        check!("hands-free", HandsFree);
        check!("portable", Portable);
        DeviceType::Default
    }

    fn vol_changed(sender: &MSender<PulseWM>, device: &Device) {
        let dt = Self::is_headphone(device);
        let muted = device.muted();
        let vol = device.volume();

        sender
            .send(PulseWM::Full {
                muted,
                vol,
                device_type: dt,
            })
            .unwrap();
    }
}

impl Block for PulseBlock {
    type Out = PulseWM;

    type In = PulseBM;

    fn run(&mut self) -> AResult<()> {
        let receiver = self.dualchannel.get_in_receiver();
        let sender = self.dualchannel.get_out_sender();
        let default_sink = self.default_sink.clone();
        let mut last_time = SystemTime::now();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match receiver.recv().await {
                    Ok(msg) => match msg {
                        PulseBM::ToggleMute => {
                            let sink = default_sink.borrow();
                            let _ = sink.toggle().await;
                        }
                        PulseBM::SetVolume(_) => {}
                        PulseBM::Increase(v) => {
                            let now = SystemTime::now();

                            if now.duration_since(last_time).unwrap_or_default()
                                > Duration::from_millis(100)
                            {
                                let sink = default_sink.borrow();
                                let _ = sink
                                    .set_volume(v as i32, Some(150))
                                    .map_err(|e| tracing::info!("error: {e}"));
                                last_time = now;
                            }
                        }
                        PulseBM::Decrease(v) => {
                            let now = SystemTime::now();

                            if now.duration_since(last_time).unwrap_or_default()
                                > Duration::from_millis(100)
                            {
                                let sink = default_sink.borrow();
                                let _ = sink
                                    .set_volume(-1 * v as i32, Some(150))
                                    .map_err(|e| tracing::info!("error: {e}"));
                                last_time = now;
                            }
                        }
                        PulseBM::GetVolume => {
                            default_sink.borrow_mut().get_info().await.unwrap();
                            Self::vol_changed(&sender, &default_sink.borrow())
                        }
                    },
                    Err(_) => {}
                }
            }
        });

        Ok(())
    }

    fn widget(&self, _: &crate::window::WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let volume = gtk::Label::builder().build();
        let headphone_icon = gtk_icon_loader::load_fixed_status_image(StatusName::Headphone);
        let headset_icon = gtk_icon_loader::load_fixed_status_image(StatusName::Headset);
        let vol_icon = gtk_icon_loader::load_fixed_status_image(StatusName::VolumeMedium);

        holder.pack_start(&headset_icon, false, false, 0);
        holder.pack_start(&headphone_icon, false, false, 0);

        holder.pack_start(&vol_icon, false, false, 0);
        holder.pack_start(&volume, false, false, 0);

        let mut receiver = self.dualchannel.get_out_receiver();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match receiver.recv().await {
                    Ok(msg) => {
                        if let PulseWM::Full {
                            muted,
                            vol,
                            device_type,
                        } = msg
                        {
                            match device_type {
                                DeviceType::Headset => {
                                    headset_icon.show();
                                    headphone_icon.hide();
                                }
                                DeviceType::Headphone => {
                                    headset_icon.hide();
                                    headphone_icon.show();
                                }
                                _ => {
                                    headset_icon.hide();
                                    headphone_icon.hide();
                                }
                            }

                            let mapped = if muted {
                                StatusName::VolumeMute
                            } else {
                                match vol {
                                    0..=30 => StatusName::VolumeLow,
                                    31..=65 => StatusName::VolumeMedium,
                                    31.. => StatusName::VolumeHigh,
                                }
                            };
                            vol_icon.set_from_surface(load_fixed_status_surface(mapped).as_ref());
                            volume.set_text(format!(" {}%", vol).as_str());
                        };
                    }
                    Err(_) => {}
                }
            }
        });
        let holder = EventBox::builder().child(&holder).build();

        let sender = self.dualchannel.in_sender.clone();
        holder.connect_scroll_event(move |_, v| {
            if let Some((_, v)) = v.scroll_deltas() {
                if v > 0.02 {
                    let _ = sender.send_blocking(PulseBM::Increase(3));
                    return Propagation::Stop;
                } else if v < -0.02 {
                    let _ = sender.send_blocking(PulseBM::Decrease(3));
                    return Propagation::Stop;
                } else {
                    Propagation::Proceed
                }
            } else {
                Propagation::Proceed
            }
        });

        let sender = self.dualchannel.in_sender.clone();
        holder.connect_button_release_event(move |_, v1| match v1.button() {
            1 => {
                let _ = sender.send_blocking(PulseBM::ToggleMute);
                Propagation::Stop
            }
            _ => Propagation::Proceed,
        });

        holder.add_events(EventMask::SCROLL_MASK | EventMask::SMOOTH_SCROLL_MASK);

        let holder = gtk::Box::builder().child(&holder).build();

        holder.upcast()
    }
}
