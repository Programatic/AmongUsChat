use anyhow::Result;

#[derive(Debug)]
pub struct Client {
    pub id: usize,
    pub stream: std::net::TcpStream,
}

pub struct Server {
    clients: Vec<Client>,
}

impl Server {
    pub fn start(&self) -> Result<()> {
        let listener = std::net::TcpListener::bind("127.0.0.1:45629")?;
        for socket in listener.incoming() {
            match socket {
                Ok(sock) => {
                    let client = Client {
                        id: 1,
                        stream: sock,
                    };
                    println!("New Client: {:?}", client);
                }
                Err(e) => {}
            }
        }

        Ok(())
    }

    pub fn new() -> Self {
        Server {
            clients: Vec::new()
        }
    }
}
