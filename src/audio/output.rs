use anyhow::Result;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream,
};
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
}

pub fn start(udp_socket: UdpSocket) -> anyhow::Result<(AudioOutput, Stream)> {
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
    };

    let stream = output_driver.start(udp_socket)?;

    Ok((output_driver, stream))
}

impl AudioOutput {
    pub fn start(&self, udp_socket: UdpSocket) -> anyhow::Result<Stream> {
        let mut decoder = magnum_opus::Decoder::new(48000, magnum_opus::Channels::Stereo)?;

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let resampler_buffs = self.resampler_buffs.clone();
        let audio_out_buffs = self.audio_out_buffs.clone();

        let _ = std::thread::spawn(move || loop {
            let mut buff = [0u8; 1500];
            let bytes = udp_socket.recv(&mut buff).unwrap();

            let id = buff[0];
            let mut out_audio_dat = [0f32; 960];
            let len = decoder
                .decode_float(&buff[1..bytes], &mut out_audio_dat, false)
                .unwrap();

            let mut resampler_buffs = resampler_buffs.lock();

            if let Some(resampler_buff) = resampler_buffs.get_mut(&id) {
                resampler_buff.extend_from_slice(&out_audio_dat[..len * 2]);
            }
        });

        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                write_data(data, &audio_out_buffs)
            },
            err_fn,
        )?;

        Ok(stream)
    }

    pub fn new_stream(&mut self, id: u8) -> Result<()> {
        println!("Creating New Stream");
        let sample_rate = self.config.sample_rate.0 as usize;
        let channels = self.config.channels as usize;

        let resampler_buffs2 = self.resampler_buffs.clone();
        let audio_out_buffs2 = self.audio_out_buffs.clone();
        let mut resampler = FftFixedInOut::<f32>::new(48000, sample_rate, 480, 1);

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
                    ao_buff.append(&mut proc);
                }

                buff.drain(..num_chunks * csize);
            }
        });

        Ok(())
    }
}

fn write_data(output: &mut [f32], audio_data: &Arc<Mutex<HashMap<u8, Vec<f32>>>>)
// where
// T: cpal::Sample,
{
    let mut audio_data = audio_data.lock();
    let mut iters = Vec::new();
    for (id, buff) in audio_data.iter_mut() {
        println!("{} {}", id, buff.len());
        let ub = if output.len() > buff.len() {
            buff.len()
        } else {
            output.len()
        };

        iters.push(buff.drain(..ub));
    }

        for sample in output.iter_mut() {
            let mut s = 0f32;
            let mut div = 0;
            for i in iters.iter_mut() {
                if let Some(val) = i.next() {
                    s += val;
                    div += 1;
                }
            }
            let value = cpal::Sample::from::<f32>(&(s/div as f32));
            *sample = value;
        }

}


