mod audio;
mod client;

use cpal::traits::StreamTrait;
use parking_lot::Mutex;
use core::panic;
use std::{collections::HashMap, io::{Read, Write}, net::UdpSocket, sync::Arc};

use client::Client;
use serde_json::json;

macro_rules! IP {
    ( $port:literal ) => {
        concat!("192.168.101.52", ":", $port)
    };
}

fn main() -> anyhow::Result<()> {
    let clients = HashMap::<u8, Client>::new();
    let clients = Arc::new(Mutex::new(clients));

    let mut socket = std::net::TcpStream::connect(IP!(45629))?;
    // let udp_socket = std::net::UdpSocket::bind("0.0.0.0:0")?;

    let mut buff = [0u8; 1];
    socket.read(&mut buff)?;
    socket.set_nonblocking(true)?;

    let id = buff[0];

    // udp_socket.connect(IP!(45628))?;
    // udp_socket.send(&Vec::from([0, id]).as_slice())?;

    let socket = Arc::new(Mutex::new(socket));
    let socket2 = socket.clone();

    // let (mut audio_driver, stream) = audio::output::start(udp_socket)?;
    let mut audio_driver = audio::output::new();
    let _stream = audio_driver.start()?; // Needed to make sure stream does not get dropped

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let json_out = json!({
            "type": "hb"
        });

        println!("Sending HB");

        let mut socket = socket2.lock();
        socket.write(json_out.to_string().as_bytes()).unwrap();
    });

    std::thread::spawn(move || -> anyhow::Result<()> {
        // let mut streams = Vec::new();
        loop {
            let mut socket = socket.lock();
            let mut buff = [0u8; 1024];
            let res = socket.read(&mut buff);
            if let Ok(bytes) = res {
                let incoming_json: serde_json::Value =
                    serde_json::from_slice(&buff[..bytes]).unwrap();
                match incoming_json["type"].as_str() {
                    Some("connect") => {
                        println!("New peer!");
                        let incoming_id = incoming_json["id"].as_u64().unwrap() as u8;
                        let new_client = Client { id: id };

                        let nsock = UdpSocket::bind("0.0.0.0:0")?;
                        nsock.connect(IP!(45628))?;

                        println!("{:#?}", nsock);

                        guaranteed_send(&nsock, &[0, id, incoming_id])?;

                        // streams.push(audio_driver.new_stream(id).unwrap());
                        audio_driver.new_stream(id, nsock)?;

                        let mut clients = clients.lock();
                        clients.insert(id, new_client);
                    }
                    Some("disconnect") => {
                        // TODO: Implement
                        println!("Peer disconnected");
                    }
                    _ => {}
                }
            }
        }
    });

    // stream.play()?;

    loop {}

    // Ok(())
}

// Should Only be used in very specific scenarios

fn guaranteed_send(udp_socket: &UdpSocket, msg: &[u8]) -> anyhow::Result<()> {
    let udp_socket = udp_socket.try_clone()?;
    udp_socket.set_nonblocking(false)?;
    udp_socket.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    println!("{:?}", msg);

    udp_socket.send_to(msg, IP!(45628))?;

    // TODO: Proper error handling
    // TODO: Checksum or some other validation
    let mut buff = [0; 3];
    let mut attmp = 0;

    while let Err(x) = udp_socket.recv(&mut buff) {
        attmp += 1;
        udp_socket.send(msg)?;

        if attmp >= 5 {
            eprintln!("Failed to Send UDP Message: {}", x);
        }
    }

    Ok(())
}