pub mod config;
pub mod error;

use std::io::Write;

use crate::core::ingestor::config::GliderNetConfig;
use crate::core::thread_manager::SteppableTask;

use crossbeam_channel;

pub trait Connection: std::io::Read + std::io::Write + Send {}
impl<T: std::io::Read + std::io::Write + Send> Connection for T {}

pub struct Ingestor<C: Connection> {
    reader: std::io::BufReader<C>,
    sender: crossbeam_channel::Sender<String>,
    writer: Option<std::io::BufWriter<std::fs::File>>,
}

impl<C: Connection> Ingestor<C> {
    fn from_connection(
        connection: C,
        sender: crossbeam_channel::Sender<String>,
        writer: Option<std::io::BufWriter<std::fs::File>>,
    ) -> Self {
        Self {
            reader: std::io::BufReader::new(connection),
            sender,
            writer,
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

                if let Some(writer) = self.writer.as_mut() {
                    let _ = write!(writer, "{line_buffer}")
                        .map_err(|error| log::error!("Failed to log to disk: {error}"));
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
        write_path: Option<&std::path::Path>,
    ) -> Result<Self, std::io::Error> {
        log::info!("Connecting to TCP stream.");
        let mut stream =
            std::net::TcpStream::connect(format!("{0}:{1}", config.host, config.port))?;

        authentication_handshake(&mut stream, &config.filter)?;

        let writer = write_path.map(create_writer).transpose()?;

        Ok(Self::from_connection(stream, sender, writer))
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

fn create_writer(
    output_path: &std::path::Path,
) -> Result<std::io::BufWriter<std::fs::File>, std::io::Error> {
    if output_path.exists() {
        std::fs::File::options()
            .append(true)
            .open(output_path)
            .map(std::io::BufWriter::new)
    } else {
        std::fs::File::create(output_path).map(std::io::BufWriter::new)
    }
}

#[cfg(test)]
mod test {

    use crate::core::ingestor::{Ingestor, create_writer};
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

        let mut ingestor = Ingestor::from_connection(mock_connection, sender, None);

        let keep_running = ingestor.step();

        assert!(keep_running);
        assert_eq!(receiver.recv().unwrap(), data);
    }

    #[test]
    fn given_connection_to_stream_when_end_of_stream_then_ingestor_stops_running() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let data = "";
        let mock_connection = MockConnection::new(data);

        let mut ingestor = Ingestor::from_connection(mock_connection, sender, None);

        let keep_running = ingestor.step();

        assert!(!keep_running);
        assert!(receiver.try_recv().is_err(), "Channel should be empty");
    }

    pub struct TestPath {
        _guard: tempfile::TempDir,
        pub path: std::path::PathBuf,
    }

    #[rstest::fixture]
    fn test_path() -> TestPath {
        let guard = tempfile::tempdir().expect("Failed to create temporary directory");
        let path = guard.path().to_path_buf();
        TestPath {
            _guard: guard,
            path,
        }
    }

    #[rstest::rstest]
    fn given_connection_to_stream_and_write_path_is_some_then_ingestor_logs_all_messages_to_path(
        test_path: TestPath,
    ) {
        let log_path = test_path.path.join("test_log.log");
        let (sender, receiver) = crossbeam_channel::unbounded();
        let data = "test";
        let mock_connection = MockConnection::new(data);
        let writer = create_writer(&log_path).expect("Failed to create writer");

        let mut ingestor = Ingestor::from_connection(mock_connection, sender, Some(writer));

        ingestor.step();

        // drop ingestor to flush writer
        drop(ingestor);
        println!("{log_path:?}");

        let file_contents = std::fs::read_to_string(&log_path).expect("Failed to read log file");

        assert_eq!(file_contents, data);
        assert_eq!(receiver.recv().unwrap(), data);
    }
}
