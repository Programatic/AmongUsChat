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
        println!("Raw {}", buff.len());
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
            if buff.len() > 0 {
            println!("Encode {}", buff.len());
            }
            let chunks_iter = buff.chunks_exact(960);
            let num_chunks = chunks_iter.len();
            for chunk in chunks_iter {
                let mut slice_u8 = encoder.encode_vec_float(chunk, 1500).unwrap();

                slice_u8.insert(0, 1);
                slice_u8.insert(1, 0);

                // println!("{:?}", Instant::now().duration_since(start));
                // todo!();

                socket
                    .send_to(&slice_u8[..], "127.0.0.1:45628")
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

// let socket = socket.lock().unwrap();
//
// todo!("Do this");
//
// fn main() -> std::io::Result<()> {
//     // let foo = Command::new("test.exe")
//     // .output().unwrap();
//
//     const PIPE: &str = "\\\\.\\pipe\\amonguspipe";
//
//     let mut f = std::fs::File::open(PIPE)?;
//
//     use std::io::Read;
//
//     let mut r: [u8; 1024] = [0; 1024];
//     while true {
//         let len = f.read(&mut r).unwrap();
//         let s = std::str::from_utf8(&mut r[..len]).unwrap();
//         let j: PlayerState = serde_json::from_str(s).unwrap();
//         println!("{:?}", j);
//     }
//
//     Ok(())
// }
//
// Maybe Get Back To
// extern "C" {
//     fn GetPid(name: *mut std::os::raw::c_char) -> u32;
//     fn injectAmongUs();
// }
// fn main() {
// //     // inject::inject_dll(inject::find_target_process("Among Us.exe"), "dll_file: &str");
// //     let foo = Command::new("C:\\Users\\Development\\Documents\\cheats\\inject\\test.exe")
// //     .output().unwrap();
//     unsafe {
//         injectAmongUs();
//
//         std::io::stdin().read_line(&mut String::new()).unwrap();
//         // let s= String::from("Among Us.exe");
//         // let s = std::ffi::CString::new(s).unwrap();
//         // println!("\n{}", GetPid(s.into_raw()));
//     }
// }
//
// fn main() -> anyhow::Result<()> {
//     let host = cpal::default_host();
//
//     let device = host.default_output_device().unwrap();
//
//     println!("Output device: {}", device.name()?);
//
// let config = device.default_output_config().unwrap();
//     println!("Default output config: {:?}", config);
//
//     match config.sample_format() {
//         cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()),
//         cpal::SampleFormat::I16 => run::<i16>(&device, &config.into()),
//         cpal::SampleFormat::U16 => run::<u16>(&device, &config.into()),
//     }
// }
//
// pub fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<(), anyhow::Error>
// where
//     T: cpal::Sample,
// {
//     const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.raw");
//     let mut f = std::fs::File::open(PATH).unwrap();
//
//     use std::io::Read;
//     let mut buff: Vec<u8> = Vec::new();
//     f.read_to_end(&mut buff);
//
//     // let mut buff = unsafe {
//     //     std::slice::from_raw_parts(
//     //         buff.as_ptr() as *const f32,
//     //         buff.len() / std::mem::size_of::<f32>(),
//     //     )
//     // }
//     // .to_vec();
//
//     println!("{:?}", buff);
//
//     todo!();
//
//     let mut decoder = magnum_opus::Decoder::new(48000, magnum_opus::Channels::Stereo)?;
//     let mut output: Vec<f32> = Vec::with_capacity(buff.len());
//
//     for chunk in buff.chunks(20) {
//         println!("Succ");
//         let mut decode_buff = [0f32; 960*2*4];
//         let out = decoder.decode_float(chunk, &mut decode_buff[..], false)?;
//         output.extend_from_slice(&decode_buff[..out]);
//     }
//     // output.truncate(out);
//
//     todo!("{}", output.len());
//
//     let mut dat_index = 0;
//
//     let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
//
//     let stream = device.build_output_stream(
//         config,
//         move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
//             write_data(data, 2, &mut dat_index, &mut output)
//         },
//         err_fn,
//     )?;
//     stream.play()?;
//
//     std::thread::sleep(std::time::Duration::from_millis(10000));
//     // drop(output);
//
//     Ok(())
// }
//
// fn write_data<'a, T>(output: &mut [T], channels: usize, data_index: &mut usize, data: &mut Vec<f32>)
// where
//     T: cpal::Sample,
// {
//     for frame in output.chunks_mut(channels) {
//         let value = data.get(*data_index);
//         if let Some(value) = value {
//             let value: T = cpal::Sample::from::<f32>(value);
//             for sample in frame.iter_mut() {
//                 *sample = value;
//                 *data_index = *data_index + 1usize;
//             }
//         }
//     }
// }
