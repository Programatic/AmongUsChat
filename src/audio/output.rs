use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use parking_lot::Mutex;
use rubato::{FftFixedInOut, Resampler};
use std::sync::Arc;

pub fn start() -> anyhow::Result<()> {
    // let sock = std::net::TcpStream::connect("127.0.0.1:45629")?;

    // return Ok(());

    let host = cpal::default_host();

    let device = host.default_output_device().unwrap();

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);

    run(&device, &mut config.into())

    // match config.sample_format() {
    //     cpal::SampleFormat::F32 => run::<f32>(&device, &mut config.into()),
    //     cpal::SampleFormat::I16 => run::<i16>(&device, &mut config.into()),
    //     cpal::SampleFormat::U16 => run::<u16>(&device, &mut config.into()),
    // }
}

fn run(device: &cpal::Device, config: &mut cpal::StreamConfig) -> Result<(), anyhow::Error>
// where
    // T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as usize;
    let channels = config.channels as usize;

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    // let socket = std::net::UdpSocket::bind("192.168.1.227:1337")?;
    let socket = std::net::UdpSocket::bind("127.0.0.1:1337")?;
    let mut decoder = magnum_opus::Decoder::new(48000, magnum_opus::Channels::Stereo)?;
    let mut resampler = FftFixedInOut::<f32>::new(48000, sample_rate, 480, 1);
    let overflow = Arc::new(Mutex::new(Vec::<f32>::with_capacity(1024)));
    let overflow2 = overflow.clone();
    let mut audio_data = Arc::new(Mutex::new(Vec::<f32>::with_capacity(1024)));
    let audio_data2 = audio_data.clone();

    let _ = std::thread::spawn(move || {
        loop {
            let mut audio_data = [0u8; 1024];
            socket.recv(&mut audio_data).unwrap();

            let num_bytes = audio_data[0];

            let mut out_audio_data = [0f32; 960];

            let len = decoder
                .decode_float(
                    &audio_data[1..(1 + num_bytes as usize)],
                    &mut out_audio_data,
                    false,
                )
                .unwrap();

            let mut overflow = overflow.lock();
            overflow.extend_from_slice(&out_audio_data[..len * 2]);

            // println!("{:?}", overflow.len());
        }
    });

    let _ = std::thread::spawn(move || loop {
        let mut overflow = overflow2.lock();
        let frames_needed = resampler.nbr_frames_needed();
        if overflow.len() >= frames_needed {
            let samples = resampler
                .process(&vec![overflow[..frames_needed].to_owned(); 1])
                .unwrap();

            let mut audio_data = audio_data2.lock();
            audio_data.append(&mut samples[0].to_owned());

            overflow.drain(..frames_needed);
        }
    });

    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut audio_data)
        },
        err_fn,
    )?;
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_secs(1000));

    Ok(())
}

fn write_data(output: &mut [f32], channels: usize, audio_data: &mut Arc<Mutex<Vec<f32>>>)
// where
    // T: cpal::Sample,
{
    let mut audio_data = audio_data.lock();
    if audio_data.len() > 0 {
        let mut audio_iter = audio_data.iter();
        let mut num_frames = 0;

        for frame in output.chunks_mut(channels) {
            for sample in frame.iter_mut() {
                if let Some(audio_val) = audio_iter.next() {
                    // println!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis());
                    // todo!();
                    let value = cpal::Sample::from::<f32>(&(*audio_val * 1f32));
                    *sample = value;
                    num_frames = num_frames + 1;
                }
            }
        }

        audio_data.drain(..num_frames);
    }
}
