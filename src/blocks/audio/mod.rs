#[allow(dead_code)]
pub mod pulse;

use crate::prelude::*;

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, SystemTime},
};

use crate::{
    datahodler::channel::{DualChannel, MSender},
    util::gtk_icon_loader::{self, load_label, IconName},
};

use self::pulse::Device;

use anyhow::Result;

use gdk::{glib::Propagation, EventMask};
use glib::MainContext;
use gtk::{
    prelude::{BoxExt, LabelExt, WidgetExt, WidgetExtManual},
    EventBox,
};

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
    fn set_volume(&self, step: i32, max_vol: Option<u32>) -> Result<()>;
    async fn toggle(&self) -> Result<()>;
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

    fn vol_changed(sender: &MSender<PulseWM>, device: &Device) {
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

    fn widget(&self, _: &crate::statusbar::WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let volume = gtk::Label::builder().build();
        let headphone_icon = gtk_icon_loader::load_font_icon(IconName::Headphone);
        let vol_icon = gtk_icon_loader::load_font_icon(IconName::VolumeMedium);

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
                                vol_icon
                                    .set_label(&load_label(gtk_icon_loader::IconName::VolumeMute))
                            } else {
                                match vol {
                                    0..=30 => vol_icon.set_label(&load_label(
                                        gtk_icon_loader::IconName::VolumeLow,
                                    )),
                                    31..=65 => vol_icon.set_label(&load_label(
                                        gtk_icon_loader::IconName::VolumeMedium,
                                    )),
                                    31.. => vol_icon.set_label(&load_label(
                                        gtk_icon_loader::IconName::VolumeHigh,
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
