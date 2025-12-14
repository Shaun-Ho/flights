use crate::thread_manager::SteppableTask;
use crossbeam_channel;
use serde;
use std::io::{BufRead, Write};

#[derive(serde::Deserialize)]
pub struct GliderNetConfig {
    pub host: String,
    pub port: u64,
    pub filter: String,
}

pub struct Ingestor {
    stream: std::net::TcpStream,
    sender: crossbeam_channel::Sender<String>,
}
impl Ingestor {
    pub fn new(
        config: &GliderNetConfig,
        sender: crossbeam_channel::Sender<String>,
    ) -> Result<Self, std::io::Error> {
        let login = format!(
            "user N0CALL pass -1 vers SkyTrace 0.1.0 filter {0} \r\n",
            config.filter
        );
        log::info!("Connecting to TCP stream.");
        let mut stream =
            std::net::TcpStream::connect(format!("{0}:{1}", config.host, config.port))?;
        stream.write_all(login.as_bytes())?;
        log::info!("Connection successful.");
        Ok(Ingestor { stream, sender })
    }
}

impl SteppableTask for Ingestor {
    fn step(&mut self) -> bool {
        log::debug!("Pulling line from ");
        let mut reader = std::io::BufReader::new(&self.stream);
        let mut line_buffer = String::new();

        reader
            .read_line(&mut line_buffer)
            .map_err(|e| {
                log::error!("{e}");
                false
            })
            .map(|bytes| {
                if bytes == 0 {
                    return false;
                }
                self.sender
                    .send(line_buffer)
                    .map_err(|e| {
                        log::error!("{e}");
                        false
                    })
                    .is_ok()
            })
            .is_ok()
    }
}
