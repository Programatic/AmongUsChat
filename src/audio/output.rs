use anyhow::Result;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream,
};
use magnum_opus::Decoder;
use parking_lot::Mutex;
use rubato::{FftFixedInOut, Resampler};
use std::{
    collections::HashMap,
    net::UdpSocket,
    sync::{atomic::AtomicBool, Arc},
};

pub struct AudioOutput {
    active_threads: HashMap<u8, AtomicBool>,
    resampler_buffs: Arc<Mutex<HashMap<u8, Vec<f32>>>>,
    audio_out_buffs: Arc<Mutex<HashMap<u8, Vec<f32>>>>,
    config: cpal::StreamConfig,
    device: Device,
    decoders: Arc<Mutex<HashMap<u8, Decoder>>>,
}

pub fn start(udp_socket: UdpSocket) -> anyhow::Result<AudioOutput> {
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);

    let output_driver = AudioOutput {
        active_threads: HashMap::new(),
        resampler_buffs: Arc::new(Mutex::new(HashMap::<u8, Vec<f32>>::with_capacity(2000))),
        audio_out_buffs: Arc::new(Mutex::new(HashMap::<u8, Vec<f32>>::with_capacity(2000))),
        device: device,
        config: config.into(),
        decoders: Arc::new(Mutex::new(HashMap::<u8, Decoder>::new())),
    };

    output_driver.start(udp_socket)?;

    Ok(output_driver)
}

impl AudioOutput {
    pub fn start(&self, udp_socket: UdpSocket) -> anyhow::Result<()> {
        let resampler_buffs = self.resampler_buffs.clone();
        let decoders = self.decoders.clone();
        // let resampler_buffs =

        let _ = std::thread::spawn(move || loop {
            let mut buff = [0u8; 1500];
            let bytes = udp_socket.recv(&mut buff).unwrap();

            let id = buff[0];
            let mut out_audio_dat = [0f32; 960];
            let mut decoders = decoders.lock();
            if let Some(decoder) = decoders.get_mut(&id) {
                let len = decoder
                    .decode_float(&buff[1..bytes], &mut out_audio_dat, false)
                    .unwrap();

                let mut resampler_buffs = resampler_buffs.lock();

                if let Some(resampler_buff) = resampler_buffs.get_mut(&id) {
                    resampler_buff.extend_from_slice(&out_audio_dat[..len * 2]);
                }
            }
        });

        Ok(())
    }

    pub fn new_stream(&mut self, id: u8) -> Result<Stream> {
        println!("Creating New Stream");
        let sample_rate = self.config.sample_rate.0 as usize;
        let channels = self.config.channels as usize;

        let resampler_buffs2 = self.resampler_buffs.clone();
        let audio_out_buffs2 = self.audio_out_buffs.clone();
        let mut resampler = FftFixedInOut::<f32>::new(48000, sample_rate, 960, 1);

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let audio_out_buffs = audio_out_buffs2.clone();

        let mut decoders = self.decoders.lock();
        decoders.insert(id, magnum_opus::Decoder::new(48000, magnum_opus::Channels::Stereo)?);
        drop(decoders);

        let mut rb = resampler_buffs2.lock();
        rb.insert(id, Vec::with_capacity(1000));
        drop(rb);

        let mut ab = audio_out_buffs2.lock();
        ab.insert(id, Vec::with_capacity(1000));
        drop(ab);

        std::thread::spawn(move || loop {
            let mut buffs = resampler_buffs2.lock();
            let csize = resampler.nbr_frames_needed();

            if let Some(buff) = buffs.get_mut(&id) {
                let iter = buff.chunks_exact(csize);
                let num_chunks = iter.len();

                let mut proc = Vec::new();

                for chunk in iter {
                    let mut processed = resampler.process(&vec![chunk.to_vec()]).unwrap();

                    proc.append(&mut processed[0]);
                }

                let mut audio_out = audio_out_buffs2.lock();
                if let Some(ao_buff) = audio_out.get_mut(&id) {
                    // println!("{}", ao_buff.len());
                    ao_buff.append(&mut proc);
                }

                buff.drain(..num_chunks * csize);
            }
        });

        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                write_data(data, channels, &audio_out_buffs, id)
            },
            err_fn,
        )?;

        stream.play()?;

        Ok(stream)
    }
}

fn write_data(
    output: &mut [f32],
    channels: usize,
    audio_data: &Arc<Mutex<HashMap<u8, Vec<f32>>>>,
    id: u8,
)
// where
// T: cpal::Sample,
{
    let mut audio_data = audio_data.lock();
    if let Some(data) = audio_data.get_mut(&id) {
        println!("{} {} {}", id, data.len(), output.len());
        let ub = if output.len() > data.len() {
            data.len()
        } else {
            output.len()
        };

        let mut iter = data.drain(..ub);

        for frame in output.chunks_mut(channels) {
            for sample in frame.iter_mut() {
                if let Some(s) = iter.next() {
                    let value = cpal::Sample::from::<f32>(&s); // Mult here for volume
                    *sample = value;
                }
            }
        }
    }
}
