//! Sound effects with `sndPlaySound(SND_ASYNC)` semantics: a new sound cuts the previous one.

use std::collections::HashMap;
use std::num::NonZero;

use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};

const SAMPLE_RATE: u32 = 11025;

pub struct Audio {
    sink: MixerDeviceSink,
    current: Option<Player>,
}

impl Audio {
    /// Returns None when no output device is available; the screensaver then runs silently.
    pub fn new() -> Option<Self> {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(mut sink) => {
                sink.log_on_drop(false);
                Some(Self { sink, current: None })
            }
            Err(e) => {
                eprintln!("warning: audio disabled: {e}");
                None
            }
        }
    }

    pub fn play(&mut self, bank: &HashMap<u32, Vec<f32>>, id: u32) {
        let Some(samples) = bank.get(&id) else { return };
        let player = Player::connect_new(self.sink.mixer());
        player.append(SamplesBuffer::new(
            NonZero::<u16>::MIN,
            NonZero::new(SAMPLE_RATE).unwrap(),
            samples.clone(),
        ));
        self.current = Some(player); // dropping the previous player stops it
    }
}
