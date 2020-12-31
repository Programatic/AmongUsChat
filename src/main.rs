mod sessions;

fn main() -> anyhow::Result<()> {
    let server = sessions::Server::new();
    server.start()?;

    Ok(())
}