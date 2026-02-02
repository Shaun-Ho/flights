pub mod config;
pub mod error;

use crate::cli::IngestorConfig;
use crate::core::ingestor::config::GliderNetConfig;
use crate::core::thread_manager::SteppableTask;

use crossbeam_channel;

use std::io::{BufRead, Write};

pub struct Ingestor {
    reader: std::io::BufReader<std::net::TcpStream>,
    sender: crossbeam_channel::Sender<String>,
    output_writer: Option<std::io::BufWriter<std::fs::File>>,
}
impl Ingestor {
    pub fn new(
        glidernet: &GliderNetConfig,
        sender: crossbeam_channel::Sender<String>,
        config: IngestorConfig,
    ) -> Result<Self, std::io::Error> {
        let login = format!(
            "user N0CALL pass -1 vers AirspaceRadar 0.1.0 filter {0} \r\n",
            glidernet.filter
        );
        log::info!("Connecting to TCP stream.");
        let mut stream =
            std::net::TcpStream::connect(format!("{0}:{1}", glidernet.host, glidernet.port))?;
        stream.write_all(login.as_bytes())?;
        log::info!("Connection successful.");
        let reader = std::io::BufReader::new(stream);

        let writer = config
            .log_input_data_stream
            .map(|output_path| {
                if output_path.exists() {
                    std::fs::File::options()
                        .append(true)
                        .open(&output_path)
                        .map(std::io::BufWriter::new)
                } else {
                    std::fs::File::create(&output_path).map(std::io::BufWriter::new)
                }
            })
            .transpose()?;

        Ok(Ingestor {
            reader,
            sender,
            output_writer: writer,
        })
    }
}

impl SteppableTask for Ingestor {
    fn step(&mut self) -> bool {
        let mut line_buffer = String::new();

        let Ok(bytes_read) = self.reader.read_line(&mut line_buffer) else {
            log::error!("Failed to read line from source");
            return true;
        };

        if let Some(output_writer) = &mut self.output_writer {
            let _ = write!(output_writer, "{line_buffer}")
                .map_err(|error| log::error!("Failed to log to disk: {error}"));
        }
        
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
