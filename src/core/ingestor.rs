pub mod config;
pub mod error;

use crate::core::ingestor::config::GliderNetConfig;
use crate::core::thread_manager::SteppableTask;

use crossbeam_channel;

pub trait Connection: std::io::Read + std::io::Write + Send {}
impl<T: std::io::Read + std::io::Write + Send> Connection for T {}

pub struct Ingestor<C: Connection> {
    reader: std::io::BufReader<C>,
    sender: crossbeam_channel::Sender<String>,
}

impl<C: Connection> Ingestor<C> {
    fn from_connection(connection: C, sender: crossbeam_channel::Sender<String>) -> Self {
        Self {
            reader: std::io::BufReader::new(connection),
            sender,
        }
    }
}

impl<C: Connection + 'static> SteppableTask for Ingestor<C> {
    fn step(&mut self) -> bool {
        let mut line_buffer = String::new();
        match std::io::BufRead::read_line(&mut self.reader, &mut line_buffer) {
            Ok(bytes_read) => {
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
            Err(error) => {
                log::error!("Failed to read line from source: {error}");
                true
            }
        }
    }
}

impl Ingestor<std::net::TcpStream> {
    pub fn new(
        config: &GliderNetConfig,
        sender: crossbeam_channel::Sender<String>,
    ) -> Result<Self, std::io::Error> {
        log::info!("Connecting to TCP stream.");
        let mut stream =
            std::net::TcpStream::connect(format!("{0}:{1}", config.host, config.port))?;

        authentication_handshake(&mut stream, &config.filter)?;

        Ok(Self::from_connection(stream, sender))
    }
}

fn authentication_handshake<W: std::io::Write>(
    writer: &mut W,
    filter: &str,
) -> std::io::Result<()> {
    let login = format!("user N0CALL pass -1 vers AirspaceRadar 0.1.0 filter {filter}\r\n",);
    writer.write_all(login.as_bytes())?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod test {

    use crate::core::ingestor::Ingestor;
    use crate::core::thread_manager::SteppableTask;

    struct MockConnection {
        incoming_data: std::io::Cursor<Vec<u8>>,
        outgoing_data: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    }
    impl MockConnection {
        pub fn new(input_data: &str) -> Self {
            Self {
                incoming_data: std::io::Cursor::new(input_data.as_bytes().to_vec()),
                outgoing_data: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }
    }
    impl std::io::Read for MockConnection {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.incoming_data.read(buf)
        }
    }

    impl std::io::Write for MockConnection {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut out = self.outgoing_data.lock().unwrap();
            out.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn given_connection_to_stream_when_data_received_then_ingestor_sends_correct_data_and_keeps_running()
     {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let data = "APRS_PACKET_DATA\n";
        let mock_connection = MockConnection::new(data);

        let mut ingestor = Ingestor::from_connection(mock_connection, sender);

        let keep_running = ingestor.step();

        assert!(keep_running);
        assert_eq!(receiver.recv().unwrap(), data);
    }

    #[test]
    fn given_connection_to_stream_when_end_of_stream_then_ingestor_stops_running() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let data = "";
        let mock_connection = MockConnection::new(data);

        let mut ingestor = Ingestor::from_connection(mock_connection, sender);

        let keep_running = ingestor.step();

        assert!(!keep_running);
        assert!(receiver.try_recv().is_err(), "Channel should be empty");
    }
}
