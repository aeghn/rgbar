#[allow(dead_code)]
pub mod pulse;

use std::{cell::RefCell, rc::Rc};

use crate::{
    datahodler::channel::{DualChannel, MSender},
    utils::gtkiconloader::{self, load_label, IconName},
};

use self::pulse::Device;

use anyhow::Result;

use glib::{Cast, MainContext};
use gtk::prelude::{BoxExt, LabelExt, WidgetExt};

use super::Block;

#[derive(Clone)]
#[allow(dead_code)]
pub enum PulseBM {
    Mute,
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
    Full(bool, u32, bool), // Muted, volume, earphone
}

trait SoundDevice {
    fn volume(&self) -> u32;
    fn muted(&self) -> bool;
    fn output_name(&self) -> String;
    fn output_description(&self) -> Option<String>;
    fn active_port(&self) -> Option<String>;
    fn form_factor(&self) -> Option<&str>;

    async fn get_info(&mut self) -> Result<()>;
    async fn set_volume(&mut self, step: i32, max_vol: Option<u32>) -> Result<()>;
    async fn toggle(&mut self) -> Result<()>;
    async fn wait_for_update(&self) -> Result<()>;
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
    default_source: Rc<Device>,
}

impl PulseBlock {
    pub fn new() -> Self {
        let dualchannel = DualChannel::new(32);
        let default_sink = Rc::new(RefCell::new(
            Device::new(pulse::DeviceKind::Sink, None).unwrap(),
        ));
        let default_source = Rc::new(Device::new(pulse::DeviceKind::Source, None).unwrap());

        PulseBlock {
            dualchannel,
            default_sink,
            default_source,
        }
    }

    fn is_headphone(device: &Device) -> bool {
        match device.form_factor() {
            // form_factor's possible values are listed at:
            // https://docs.rs/libpulse-binding/2.25.0/libpulse_binding/proplist/properties/constant.DEVICE_FORM_FACTOR.html
            Some("headset") | Some("headphone") | Some("hands-free") | Some("portable") => true,
            // Per discussion at
            // https://github.com/greshake/i3status-rust/pull/1363#issuecomment-1046095869,
            // some sinks may not have the form_factor property, so we should fall back to the
            // active_port if that property is not present.
            None => device
                .active_port()
                .is_some_and(|p| p.contains("headphones")),
            // form_factor is present and is some non-headphone value
            _ => false,
        }
    }

    fn handle_update(sender: &MSender<PulseWM>, device: &Device) {
        let is_headphone = Self::is_headphone(device);
        let is_muted = device.muted();
        let volume = device.volume();

        sender
            .send(PulseWM::Full(is_muted, volume, is_headphone))
            .unwrap();
    }
}

impl Block for PulseBlock {
    type Out = PulseWM;

    type In = PulseBM;

    fn run(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();
        let default_sink = self.default_sink.clone();
        MainContext::ref_thread_default().spawn_local(async move {
            let default_sink = default_sink.clone();
            loop {
                let sink = default_sink.borrow().wait_for_update().await;
                if sink.is_ok() {
                    default_sink.borrow_mut().get_info().await.unwrap();
                    Self::handle_update(&sender, &default_sink.borrow())
                }
            }
        });

        let receiver = self.dualchannel.get_in_recevier();
        let sender = self.dualchannel.get_out_sender();
        let default_sink = self.default_sink.clone();
        MainContext::ref_thread_default().spawn_local(async move {
            let default_sink = default_sink.clone();
            loop {
                match receiver.recv().await {
                    Ok(msg) => match msg {
                        PulseBM::Mute => todo!(),
                        PulseBM::SetVolume(_) => todo!(),
                        PulseBM::Increase(_) => todo!(),
                        PulseBM::Decrease(_) => todo!(),
                        PulseBM::GetVolume => {
                            let sink = default_sink.borrow();
                            Self::handle_update(&sender, &sink)
                        }
                    },
                    Err(_) => {}
                }
            }
        });

        Ok(())
    }

    fn widget(&self, _: &crate::statusbar::WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let volume = gtk::Label::builder().build();
        let headphone_icon = gtkiconloader::load_font_icon(IconName::Headphone);
        let vol_icon = gtkiconloader::load_font_icon(IconName::VolumeMidium);

        holder.pack_start(&headphone_icon, false, false, 0);

        holder.pack_start(&vol_icon, false, false, 0);
        holder.pack_start(&volume, false, false, 0);

        let mut receiver = self.dualchannel.get_out_receiver();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match receiver.recv().await {
                    Ok(msg) => {
                        if let PulseWM::Full(mute, vol, headphone) = msg {
                            if headphone {
                                if !headphone_icon.is_visible() {
                                    headphone_icon.show();
                                }
                            } else {
                                headphone_icon.hide();
                            }

                            if mute {
                                vol_icon.set_label(&load_label(gtkiconloader::IconName::VolumeMute))
                            } else {
                                match vol {
                                    0..=30 => vol_icon
                                        .set_label(&load_label(gtkiconloader::IconName::VolumeLow)),
                                    31..=65 => vol_icon.set_label(&load_label(
                                        gtkiconloader::IconName::VolumeMidium,
                                    )),
                                    31.. => vol_icon.set_label(&load_label(
                                        gtkiconloader::IconName::VolumeHigh,
                                    )),
                                }
                            }

                            volume.set_text(format!(" {}%", vol).as_str());
                        };
                    }
                    Err(_) => {}
                }
            }
        });

        holder.upcast()
    }
}
