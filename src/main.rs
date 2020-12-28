#![allow(non_snake_case)]

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Data, DefaultStreamConfigError,
};
use magnum_opus::{Decoder, Encoder};
use rubato::Resampler;
use serde::Deserialize;
use std::fs::File;
use std::io::BufWriter;
use std::process::Command;
use std::sync::{Arc, Mutex};

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
    let mut decoder = magnum_opus::Decoder::new(48000, magnum_opus::Channels::Stereo)?;

    let mut resampler = rubato::FftFixedInOut::<f32>::new(44100, 48000, 896, 2);

    let host = cpal::default_host();

    let device = host.default_input_device().unwrap();

    println!("Input device: {}", device.name()?);

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");

    println!("Default input config: {:?}", config);

    println!("Begin recording...");

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let sample_format = config.sample_format();

    let mut socket = std::net::UdpSocket::bind("192.168.1.82:1337")?;
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.raw");
    // let socket = std::fs::File::create(PATH).unwrap();
    // let mut socket = BufWriter::new(socket);

    let stream = device
        .build_input_stream_raw(
            &config.into(),
            sample_format,
            move |data, _: &_| write_input_data_f32(data, &mut encoder, &mut decoder, &mut socket),
            err_fn,
        )
        .unwrap();

    stream.play()?;

    std::thread::sleep(std::time::Duration::from_secs(7200));
    drop(stream);

    Ok(())
}

type ResamplerHandle = rubato::FftFixedInOut<f32>;

fn write_input_data_f32(
    input: &Data,
    encoder: &mut Encoder,
    decoder: &mut Decoder,
    socket: &mut std::net::UdpSocket,
) {
    let mut inp = input.as_slice::<f32>().unwrap().to_vec();

    // inp.append(&mut vec![0f32; 960 - inp.len()]);

    // inp.truncate(resampler.nbr_frames_needed());
    // if inp.len() < resampler.nbr_frames_needed() {
    //     inp.append(&mut vec![0f32; resampler.nbr_frames_needed() - inp.len()]);
    // }
    // let mut wave_out = resampler.process(&vec![Vec::from(inp); 2]).unwrap();//[0].to_owned();

    // use itertools::interleave;
    // let v1 = wave_out[0].to_owned();
    // let v2 = wave_out[1].to_owned();
    // let v = interleave(v1.chunks(1), v2.chunks(1)).flatten().copied().collect::<Vec<f32>>();

    // let buff = encoder.encode_vec_float(v.as_slice(), 960).unwrap();
    // let buff = encoder.encode_vec_float(&inp[..], 960).unwrap();

    // let mut o = Vec::with_capacity(960);
    // let mut o = [0f32; 960];
    // decoder.decode_float(&buff[..], &mut o[..], false);

    // println!("{:?} \n\n\n\n {:?}", inp, o);

    // todo!();

    let slice_u8 = encoder.encode_vec_float(&inp, 1500).unwrap();

    // let slice_u8: &[u8] = unsafe {
    //     std::slice::from_raw_parts(inp.as_ptr() as *const u8, inp.len() * std::mem::size_of::<f32>())
    // };

    // println!("{:?}", slice_u8.len());

    // todo!();

    let mut b = [0u8; 1];
    b[0] = slice_u8.len() as u8;

    // let b = unsafe {
    //     std::slice::from_raw_parts(b.as_ptr() as *const u8, b.len() * 2)
    // };
    
    socket.send_to(&b, "192.168.1.227:1337").expect("Failure to send 1");
    socket.send_to(&slice_u8[..], "192.168.1.227:1337").expect("Failure to send 2");
    // todo!();
}

// let socket = socket.lock().unwrap();


// todo!("Do this");

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










// fn main() -> anyhow::Result<()> {
//     let host = cpal::default_host();

//     let device = host.default_output_device().unwrap();

//     println!("Output device: {}", device.name()?);

//     let config = device.default_output_config().unwrap();
//     println!("Default output config: {:?}", config);

//     match config.sample_format() {
//         cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()),
//         cpal::SampleFormat::I16 => run::<i16>(&device, &config.into()),
//         cpal::SampleFormat::U16 => run::<u16>(&device, &config.into()),
//     }
// }

// pub fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<(), anyhow::Error>
// where
//     T: cpal::Sample,
// {
//     const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.raw");
//     let mut f = std::fs::File::open(PATH).unwrap();

//     use std::io::Read;
//     let mut buff: Vec<u8> = Vec::new();
//     f.read_to_end(&mut buff);

//     // let mut buff = unsafe {
//     //     std::slice::from_raw_parts(
//     //         buff.as_ptr() as *const f32,
//     //         buff.len() / std::mem::size_of::<f32>(),
//     //     )
//     // }
//     // .to_vec();

//     println!("{:?}", buff);

//     todo!();

//     let mut decoder = magnum_opus::Decoder::new(48000, magnum_opus::Channels::Stereo)?;
//     let mut output: Vec<f32> = Vec::with_capacity(buff.len());

//     for chunk in buff.chunks(20) {
//         println!("Succ");
//         let mut decode_buff = [0f32; 960*2*4];
//         let out = decoder.decode_float(chunk, &mut decode_buff[..], false)?;
//         output.extend_from_slice(&decode_buff[..out]);
//     }
//     // output.truncate(out);

//     todo!("{}", output.len());

//     let mut dat_index = 0;

//     let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

//     let stream = device.build_output_stream(
//         config,
//         move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
//             write_data(data, 2, &mut dat_index, &mut output)
//         },
//         err_fn,
//     )?;
//     stream.play()?;

//     std::thread::sleep(std::time::Duration::from_millis(10000));
//     // drop(output);

//     Ok(())
// }

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
