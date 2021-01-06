mod audio;
mod client;

use parking_lot::Mutex;
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::Arc,
};

use client::Client;
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let clients = HashMap::<u8, Client>::new();
    let clients = Arc::new(Mutex::new(clients));

    let mut socket = std::net::TcpStream::connect("127.0.0.1:45629")?;
    let udp_socket = std::net::UdpSocket::bind("0.0.0.0:0")?;

    let mut buff = [0u8; 1];
    socket.read(&mut buff)?;
    socket.set_nonblocking(true)?;
    let id = buff[0];

    udp_socket.connect("127.0.0.1:45628")?;
    udp_socket.send(&Vec::from([0, id]).as_slice())?;

    let socket = Arc::new(Mutex::new(socket));
    let socket2 = socket.clone();

    let mut decode_buffs = HashMap::<u8, Vec<f32>>::new();

    let _ = std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let mut socket = socket2.lock();
        let json_out = json!({
            "type": "hb"
        });

        println!("Sending HB");
        socket.write(json_out.to_string().as_bytes()).unwrap();
    });

    let _ = std::thread::spawn(move || loop {
        let mut socket = socket.lock();
        let mut buff = [0u8; 1024];
        let res = socket.read(&mut buff);
        if let Ok(bytes) = res {
            let incoming_json: serde_json::Value = serde_json::from_slice(&buff[..bytes]).unwrap();
            match incoming_json["type"].as_str() {
                Some("connect") => {
                    println!("New peer!");
                    let id = incoming_json["id"].as_u64().unwrap() as u8;
                    let new_client = Client { id: id };

                    let mut clients = clients.lock();
                    clients.insert(id, new_client);
                },
                Some("disconnect") => {
                    println!("Peer disconnected");
                },
                _ => {}
            }
        }
    });

    let _ = std::thread::spawn(move || loop {
        let mut buff = [0u8; 1500];
        let bytes = udp_socket.recv(&mut buff).unwrap();

        println!("{:?}", &buff[..bytes]);

        std::thread::sleep(std::time::Duration::from_secs(1));
    });

    loop {}

    // Ok(())
}
