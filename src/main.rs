use std::process::Command;

use serde::Deserialize;

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

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    DefaultStreamConfigError,
};
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct Opt {
    #[cfg(all(
        any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd"),
        feature = "jack"
    ))]
    jack: bool,

    device: String,
}

impl Opt {
    fn from_args() -> Self {
        let app = clap::App::new("beep").arg_from_usage("[DEVICE] 'The audio device to use'");

        let matches = app.get_matches();
        let device = matches.value_of("DEVICE").unwrap_or("default").to_string();

        #[cfg(any(
            not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd")),
            not(feature = "jack")
        ))]
        Opt { device }
    }
}

fn main() -> Result<(), anyhow::Error> {
    let foo = Command::new("test.exe").output().unwrap();

    const PIPE: &str = "\\\\.\\pipe\\amonguspipe";
    let mut f: std::fs::File;
    loop {
        let temp = std::fs::File::open(PIPE);
        if let Ok(res) = temp {
            f = res;
            break;
        } else if let Err(x) = temp {
            println!("{}", x);
        }
    }

    let mut currentPlayerState: Arc<Mutex<PlayerState>> =
        Arc::new(Mutex::new(PlayerState::default()));
    let mut currentPlayerState2 = currentPlayerState.clone();
    let mut currentPlayerState3 = currentPlayerState.clone();

    let h1 = std::thread::spawn(move || {
        use std::io::Read;

        let mut r: [u8; 1024] = [0; 1024];
        while true {
            let len = f.read(&mut r).unwrap();
            let s = std::str::from_utf8(&mut r[..len]).unwrap();
            let currentPlayerStateInbound: PlayerState = serde_json::from_str(s).unwrap();
            let mut stateLock = currentPlayerState2.lock().unwrap();
            *stateLock = currentPlayerStateInbound;
        }
    });

    let h2 = std::thread::spawn(move || {
        while true {
            let lock = currentPlayerState3.lock().unwrap();
            println!("Player State from Thread: {:?}", lock);
            drop(lock);
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    let enc = opus::Encoder::new(48000, opus::Channels::Stereo, opus::Application::Voip);

    // std::thread::sleep(std::time::Duration::from_secs(10));

    // return Ok(());

    let opt = Opt::from_args();

    #[cfg(any(
        not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd")),
        not(feature = "jack")
    ))]
    let host = cpal::default_host();

    // Setup the input device and stream with the default input config.
    let device = if opt.device == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == opt.device).unwrap_or(false))
    }
    .expect("failed to find input device");

    println!("Input device: {}", device.name()?);

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");
    println!("Default input config: {:?}", config);

    // The WAV file we're recording to.
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
    // let spec = wav_spec_from_config(&config);
    // let writer = hound::WavWriter::create(PATH, spec)?;
    let encoder = Arc::new(Mutex::new(Some(writer)));

    // A flag to indicate that recording is in progress.
    println!("Begin recording...");

    // Run the input stream on a separate thread.
    let encoder_2 = encoder.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32>(data, &writer_2),
            err_fn,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16>(data, &writer_2),
            err_fn,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<u16>(data, &writer_2),
            err_fn,
        )?,
    };

    stream.play()?;

    // Let recording go for roughly three seconds.
    std::thread::sleep(std::time::Duration::from_secs(10));
    drop(stream);
    writer.lock().unwrap().take().unwrap().finalize()?;
    println!("Recording {} complete!", PATH);

    Ok(())
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T>(input: &[T], writer: &WavWriterHandle)
where
    T: cpal::Sample,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = cpal::Sample::from(&sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}

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
