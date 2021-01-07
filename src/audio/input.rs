// use cpal::{
//     traits::{DeviceTrait, HostTrait, StreamTrait},
//     Data, DefaultStreamConfigError,
// };
// use magnum_opus::{Decoder, Encoder};
// use parking_lot::Mutex;
// use rubato::Resampler;
// use serde::Deserialize;
// use std::process::Command;
// use std::sync::Arc;
// use std::{collections::VecDeque, fs::File};
// use std::{io::BufWriter, time::Instant};

// fn main() -> Result<(), anyhow::Error> {
//     let mut encoder = magnum_opus::Encoder::new(
//         48000,
//         magnum_opus::Channels::Stereo,
//         magnum_opus::Application::Voip,
//     )
//     .unwrap();
//     let host = cpal::default_host();

//     let device = host.default_input_device().unwrap();

//     println!("Input device: {}", device.name()?);

//     let config = device
//         .default_input_config()
//         .expect("Failed to get default input config");

//     let sample_rate = config.sample_rate().0;

//     let mut resampler = rubato::FftFixedOut::<f32>::new(sample_rate as usize, 48000, 960, 1, 1);

//     let _t1 = std::thread::spawn(move || loop {
//         let mut buff = raw_buff2.lock();
//         let mut enc_buff = Vec::<f32>::with_capacity(960);
//         let chunks_iter = buff.chunks_exact(resampler.nbr_frames_needed());
//         let num_chunks = chunks_iter.len();
//         for chunk in chunks_iter {
//             let sampled = resampler.process(&vec![chunk.into(); 1]).unwrap();

//             let mut b = sampled[0].to_owned();
//             enc_buff.append(&mut b);
//         }

//         buff.drain(..resampler.nbr_frames_needed() * num_chunks);
//         drop(buff);
//         let mut encode_buff = encode_buff2.lock();
//         encode_buff.append(&mut enc_buff);
//     });

//     let _t2 = std::thread::spawn(move || {
//         loop {
//             let mut buff = encode_buff.lock();
//             let chunks_iter = buff.chunks_exact(960);
//             let num_chunks = chunks_iter.len();
//             for chunk in chunks_iter {
//                 let mut slice_u8 = encoder.encode_vec_float(chunk, 1500).unwrap();

//                 slice_u8.insert(0, slice_u8.len() as u8);

//                 // println!("{:?}", Instant::now().duration_since(start));
//                 // todo!();

//                 socket
//                     .send_to(&slice_u8[..], "127.0.0.1:1337")
//                     .expect("Failure to send 2");
//             }
//             buff.drain(..960 * num_chunks);
//         }
//     });

//     println!("Default input config: {:?}", config);

//     println!("Begin recording...");

//     let err_fn = move |err| {
//         eprintln!("an error occurred on stream: {}", err);
//     };

//     let sample_format = config.sample_format();

//     let stream = device
//         .build_input_stream_raw(
//             &config.into(),
//             sample_format,
//             move |data, _: &_| {
//                 write_input_data_f32(data, &raw_buff);
//             },
//             err_fn,
//         )
//         .unwrap();

//     stream.play()?;

//     std::thread::sleep(std::time::Duration::from_secs(7200));
//     drop(stream);

//     Ok(())
// }

// fn write_input_data_f32(input: &Data, raw_buff: &Arc<Mutex<Vec<f32>>>) {
//     let mut inp = input.as_slice::<f32>().unwrap().to_vec();

//     let mut raw = raw_buff.lock();
//     raw.append(&mut inp);
// }
