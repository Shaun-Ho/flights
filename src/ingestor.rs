use std::io::{BufRead, Write};
pub struct Ingestor {
    host: String,
    port: u64,
}
impl Ingestor {
    #[must_use]
    pub fn new(host: String, port: u64) -> Self {
        Ingestor { host, port }
    }
    #[allow(clippy::needless_pass_by_value)]
    pub fn listen(&self, tx_external: crossbeam_channel::Sender<String>, filter: &str) {
        let login = format!("user N0CALL pass -1 vers SkyTrace 0.1.0 filter {filter} \r\n");

        match std::net::TcpStream::connect(format!("{0}:{1}", &self.host, self.port)) {
            Ok(mut stream) => {
                let _ = stream.write_all(login.as_bytes());

                let mut reader = std::io::BufReader::new(stream);
                let mut line_buffer = String::new();

                loop {
                    match reader.read_line(&mut line_buffer) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                log::info!("Connection closed by remote host.");
                                break;
                            }
                            if !line_buffer.trim().is_empty() && !line_buffer.starts_with('#') {
                                // Send the *trimmed* line to the external channel
                                let _ = tx_external.send(line_buffer.trim().to_string());
                            }
                            line_buffer.clear();
                        }
                        Err(e) => {
                            log::error!("Error reading line: {e}");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Network Error: {e}. Retrying...");
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        }
    }
}
