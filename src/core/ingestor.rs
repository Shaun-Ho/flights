pub mod config;
pub mod error;

use crate::core::ingestor::config::GliderNetConfig;
use crate::core::thread_manager::SteppableTask;

use crossbeam_channel;

use std::io::{BufRead, Write};

pub struct Ingestor {
    reader: std::io::BufReader<std::net::TcpStream>,
    sender: crossbeam_channel::Sender<String>,
}
impl Ingestor {
    pub fn new(
        config: &GliderNetConfig,
        sender: crossbeam_channel::Sender<String>,
    ) -> Result<Self, std::io::Error> {
        let login = format!(
            "user N0CALL pass -1 vers AirspaceRadar 0.1.0 filter {0} \r\n",
            config.filter
        );
        log::info!("Connecting to TCP stream.");
        let mut stream =
            std::net::TcpStream::connect(format!("{0}:{1}", config.host, config.port))?;
        stream.write_all(login.as_bytes())?;
        log::info!("Connection successful.");
        let reader = std::io::BufReader::new(stream);
        Ok(Ingestor { reader, sender })
    }
}

impl SteppableTask for Ingestor {
    fn step(&mut self) -> bool {
        let mut line_buffer = String::new();

        let Ok(bytes_read) = self.reader.read_line(&mut line_buffer) else {
            log::error!("Failed to read line from source");
            return true;
        };

        if bytes_read == 0 {
            log::error!("End of TCP stream");
            return false;
        }
        if let Err(err) = self.sender.send(line_buffer) {
            log::error!("Ingestor: Failed to send to channel: {err}");
            return false;
        }
        true
    }
}
