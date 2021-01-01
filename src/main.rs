use std::{
    io::Read,
    sync::{Arc, Mutex},
};

fn main() -> anyhow::Result<()> {
    let mut socket = std::net::TcpStream::connect("127.0.0.1:45629")?;

    let mut buff = [0u8; 1];
    socket.read(&mut buff)?;

    let id = buff[0];

    let socket = Arc::new(Mutex::new(socket));
    let socket2 = socket.clone();

    let _ = std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        if let Ok(socket) = socket2.lock() {
            todo!("Send heartbeat");
        }
    });

    Ok(())
}
