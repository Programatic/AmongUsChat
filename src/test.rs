#![allow(non_snake_case)]

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Data, DefaultStreamConfigError,
};
use magnum_opus::{Decoder, Encoder};
use parking_lot::Mutex;
use rubato::Resampler;
use serde::Deserialize;
use std::process::Command;
use std::sync::Arc;
use std::{collections::VecDeque, fs::File};
use std::{io::BufWriter, time::Instant};

#[derive(Debug, Deserialize)]
struct PlayerState {
    isCommsSabotaged: bool,
    isDead: bool,
    colorID: u8,
    hatID: isize,
    petID: isize,
    skinID: isize,
    isDisconnected: bool,
    isImpostor: bool,
    inVent: bool,
    x: f32,
    y: f32,
    name: String,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            isCommsSabotaged: false,
            isDead: false,
            colorID: 0,
            hatID: -1,
            petID: -1,
            skinID: -1,
            isDisconnected: true,
            isImpostor: false,
            inVent: false,
            x: 0f32,
            y: 0f32,
            name: String::from(""),
        }
    }
}

struct SendData<'a> {
    pState: &'a PlayerState,
    audioData: bool,
}

macro_rules! IP {
    ( $port:literal ) => {
        concat!("192.168.1.227", ":", $port)
    };
}

fn main() -> Result<(), anyhow::Error> {
    // let foo = Command::new("test.exe").output().unwrap();
    //
    // const PIPE: &str = "\\\\.\\pipe\\amonguspipe";
    // let mut f: std::fs::File;
    // loop {
    //     let temp = std::fs::File::open(PIPE);
    //     if let Ok(res) = temp {
    //         f = res;
    //         break;
    //     } else if let Err(x) = temp {
    //         println!("{}", x);
    //     }
    // }
    //
    // let mut currentPlayerState: Arc<Mutex<PlayerState>> =
    //     Arc::new(Mutex::new(PlayerState::default()));
    // let mut currentPlayerState2 = currentPlayerState.clone();
    // let mut currentPlayerState3 = currentPlayerState.clone();
    //
    // let h1 = std::thread::spawn(move || {
    //     use std::io::Read;
    //
    //     let mut r: [u8; 1024] = [0; 1024];
    //     while true {
    //         let len = f.read(&mut r).unwrap();
    //         let s = std::str::from_utf8(&mut r[..len]).unwrap();
    //         let currentPlayerStateInbound: PlayerState = serde_json::from_str(s).unwrap();
    //         let mut stateLock = currentPlayerState2.lock().unwrap();
    //         *stateLock = currentPlayerStateInbound;
    //     }
    // });
    //
    // let h2 = std::thread::spawn(move || {
    //     while true {
    //         let lock = currentPlayerState3.lock().unwrap();
    //         println!("Player State from Thread: {:?}", lock);
    //         drop(lock);
    //         std::thread::sleep(std::time::Duration::from_secs(1));
    //     }
    // });

    println!("Re-enable");

    let mut socket = std::net::TcpStream::connect(IP!(45629))?;

    use std::io::Read;
    let mut buff = [0u8; 1];
    socket.read(&mut buff);

    let id = buff[0];

    println!("{}", buff[0]);

    // panic!();

    let mut encoder = magnum_opus::Encoder::new(
        48000,
        magnum_opus::Channels::Stereo,
        magnum_opus::Application::Voip,
    )
    .unwrap();
    let host = cpal::default_host();

    let device = host.default_input_device().unwrap();

    println!("Input device: {}", device.name()?);

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");

    let sample_rate = config.sample_rate().0;

    let mut resampler = rubato::FftFixedOut::<f32>::new(sample_rate as usize, 48000, 960, 1, 1);

    // let mut socket = std::net::UdpSocket::bind("192.168.1.82:1337")?;
    let mut socket = std::net::UdpSocket::bind("0.0.0.0:0")?;

    let raw_buff = Arc::new(Mutex::new(Vec::<f32>::with_capacity(2000)));
    let raw_buff2 = raw_buff.clone();

    let encode_buff = Arc::new(Mutex::new(Vec::<f32>::with_capacity(2000)));
    let encode_buff2 = encode_buff.clone();

    // let start = Instant::now();
    // println!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis());

    let _t1 = std::thread::spawn(move || loop {
        let mut buff = raw_buff2.lock();
        // println!("Raw {}", buff.len());
        let mut enc_buff = Vec::<f32>::with_capacity(960);
        let chunks_iter = buff.chunks_exact(resampler.nbr_frames_needed());
        let num_chunks = chunks_iter.len();
        for chunk in chunks_iter {
            let sampled = resampler.process(&vec![chunk.into(); 1]).unwrap();

            let mut b = sampled[0].to_owned();
            enc_buff.append(&mut b);
        }

        buff.drain(..resampler.nbr_frames_needed() * num_chunks);
        drop(buff);
        let mut encode_buff = encode_buff2.lock();
        encode_buff.append(&mut enc_buff);
    });

    let _t2 = std::thread::spawn(move || {
        loop {
            let mut buff = encode_buff.lock();
            // if buff.len() > 0 {
            // println!("Encode {}", buff.len());
            // }
            let chunks_iter = buff.chunks_exact(960);
            let num_chunks = chunks_iter.len();
            for chunk in chunks_iter {
                let mut slice_u8 = encoder.encode_vec_float(chunk, 1500).unwrap();

                slice_u8.insert(0, 1);
                slice_u8.insert(1, id);

                // println!("{:?}", Instant::now().duration_since(start));
                // todo!();

                socket
                    .send_to(&slice_u8[..], IP!(45628))
                    // .send_to(&slice_u8, IP!(44444))
                    .expect("Failure to send 2");
            }
            buff.drain(..960 * num_chunks);
        }
    });

    println!("Default input config: {:?}", config);

    println!("Begin recording...");

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let sample_format = config.sample_format();

    let stream = device
        .build_input_stream_raw(
            &config.into(),
            sample_format,
            move |data, _: &_| {
                write_input_data_f32(data, &raw_buff);
            },
            err_fn,
        )
        .unwrap();

    stream.play()?;

    loop {}
}

fn write_input_data_f32(input: &Data, raw_buff: &Arc<Mutex<Vec<f32>>>) {
    let mut inp = input.as_slice::<f32>().unwrap().to_vec();

    let mut raw = raw_buff.lock();
    raw.append(&mut inp);
}
