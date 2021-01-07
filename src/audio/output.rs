use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use rubato::{FftFixedInOut, Resampler};
use std::{collections::HashMap, net::UdpSocket, sync::Arc};

pub fn start(udp_socket: UdpSocket) -> anyhow::Result<()> {
    let host = cpal::default_host();

    let device = host.default_output_device().unwrap();

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);


    std::thread::spawn(move || -> anyhow::Result<()> {
        run(&device, &mut config.into(), udp_socket)
    });

    // match config.sample_format() {
    //     cpal::SampleFormat::F32 => run::<f32>(&device, &mut config.into()),
    //     cpal::SampleFormat::I16 => run::<i16>(&device, &mut config.into()),
    //     cpal::SampleFormat::U16 => run::<u16>(&device, &mut config.into()),
    // }

    Ok(())
}

fn run(
    device: &cpal::Device,
    config: &mut cpal::StreamConfig,
    udp_socket: UdpSocket,
) -> Result<(), anyhow::Error>
// where
    // T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as usize;
    let channels = config.channels as usize;

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let mut decoder = magnum_opus::Decoder::new(48000, magnum_opus::Channels::Stereo)?;
    let mut resampler = FftFixedInOut::<f32>::new(48000, sample_rate, 480, 1);

    let resampler_buffs = Arc::new(Mutex::new(HashMap::<u8, Vec<f32>>::with_capacity(2000)));
    let resampler_buffs2 = resampler_buffs.clone();
    let audio_out_buffs = Arc::new(Mutex::new(HashMap::<u8, Vec<f32>>::with_capacity(2000)));
    let audio_out_buffs2 = audio_out_buffs.clone();

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
            resampler_buff.extend_from_slice(&out_audio_dat[..len*2]);
        } else {
            resampler_buffs.insert(id, Vec::from(out_audio_dat));
        }
    });

    let _ = std::thread::spawn(move || loop {
        let mut buffs = resampler_buffs2.lock();
        let mut audio_out = audio_out_buffs2.lock();
        for (id, buff) in buffs.iter_mut() {
            let iter = buff.chunks_exact(resampler.nbr_frames_needed());
            let num_chunks = iter.len();
            for chunk in iter {
                let mut processed = resampler.process(&vec![chunk.to_vec()]).unwrap()[0].to_owned();
                if let Some(ao) = audio_out.get_mut(id) {
                    ao.append(&mut processed);
                } else {
                    audio_out.insert(*id, Vec::from(processed));
                }
            }

            buff.drain(..num_chunks * resampler.nbr_frames_needed());
        }
    });

    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &audio_out_buffs)
        },
        err_fn,
    )?;
    stream.play()?;

    loop {}
}

fn write_data(output: &mut [f32], channels: usize, audio_data: &Arc<Mutex<HashMap<u8, Vec<f32>>>>)
// where
// T: cpal::Sample,
{
    let mut audio_data = audio_data.lock();
    let mut iters = Vec::new();
    for (_, buff) in audio_data.iter_mut() {
        let ub = if output.len() > buff.len() {
            buff.len()
        } else {
            output.len()
        };

        iters.push(buff.drain(..ub));
    }

    for frame in output.chunks_mut(channels) {
        for sample in frame.iter_mut() {
            let mut s = 0f32;
            for i in iters.iter_mut() {
                if let Some(val) = i.next() {
                    s += val;
                }
            }
            let value = cpal::Sample::from::<f32>(&(s * 1f32));
            *sample = value;
        }
    }

}
