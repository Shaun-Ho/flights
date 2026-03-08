pub mod config;
pub mod error;

use std::io::Write;

use crate::core::ingestor::config::GliderNetConfig;
use crate::core::thread_manager::SteppableTask;

use crossbeam_channel;

pub trait DataSource: std::io::Read + Send {}
impl<T: std::io::Read + Send> DataSource for T {}

pub struct Ingestor {
    reader: std::io::BufReader<Box<dyn DataSource>>,
    sender: crossbeam_channel::Sender<String>,
    writer: Option<std::io::BufWriter<std::fs::File>>,
}

impl Ingestor {
    pub fn new<C: DataSource + 'static>(
        source: C,
        sender: crossbeam_channel::Sender<String>,
        writer: Option<std::io::BufWriter<std::fs::File>>,
    ) -> Self {
        Self {
            reader: std::io::BufReader::new(Box::new(source)),
            sender,
            writer,
        }
    }

    pub fn read_data_from_file(
        read_path: &std::path::Path,
        sender: crossbeam_channel::Sender<String>,
        write_path: Option<&std::path::Path>,
    ) -> Result<Self, std::io::Error> {
        log::info!(
            "Reading APRS data from file: {}",
            read_path.to_string_lossy()
        );
        let source = FileDataSource::new(read_path)?;
        let writer = write_path.map(create_writer).transpose()?;
        Ok(Self::new(source, sender, writer))
    }

    pub fn connect_glidernet(
        config: &GliderNetConfig,
        sender: crossbeam_channel::Sender<String>,
        write_path: Option<&std::path::Path>,
    ) -> Result<Self, std::io::Error> {
        log::info!("Connecting to TCP stream.");
        let mut stream =
            std::net::TcpStream::connect(format!("{0}:{1}", config.host, config.port))?;

        authentication_handshake(&mut stream, &config.filter)?;

        let writer = write_path.map(create_writer).transpose()?;
        Ok(Self::new(stream, sender, writer))
    }
}
struct FileDataSource {
    pub reader: std::fs::File,
}
impl FileDataSource {
    pub fn new(input_path: &std::path::Path) -> Result<Self, std::io::Error> {
        let reader = std::fs::File::options().read(true).open(input_path)?;
        Ok(Self { reader })
    }
}
impl std::io::Read for FileDataSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

impl SteppableTask for Ingestor {
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

    use crate::core::ingestor::{FileDataSource, Ingestor, create_writer};
    use crate::core::thread_manager::SteppableTask;
    use crate::test_utilities::{TestPath, test_data_path, test_path};

    struct MockStream {
        incoming_data: std::io::Cursor<Vec<u8>>,
        outgoing_data: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    }
    impl MockStream {
        pub fn new(input_data: &str) -> Self {
            Self {
                incoming_data: std::io::Cursor::new(input_data.as_bytes().to_vec()),
                outgoing_data: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }
    }
    impl std::io::Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.incoming_data.read(buf)
        }
    }

    impl std::io::Write for MockStream {
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
        let mock_stream = MockStream::new(data);

        let mut ingestor = Ingestor::new(mock_stream, sender, None);

        let keep_running = ingestor.step();

        assert!(keep_running);
        assert_eq!(receiver.recv().unwrap(), data);
    }

    #[test]
    fn given_connection_to_stream_when_end_of_stream_then_ingestor_stops_running() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let data = "";
        let mock_stream = MockStream::new(data);

        let mut ingestor = Ingestor::new(mock_stream, sender, None);

        let keep_running = ingestor.step();

        assert!(!keep_running);
        assert!(receiver.try_recv().is_err(), "Channel should be empty");
    }

    #[rstest::rstest]
    fn given_connection_to_stream_and_write_path_is_some_then_ingestor_logs_all_messages_to_path(
        test_path: TestPath,
    ) {
        let log_path = test_path.path.join("test_log.log");
        let (sender, receiver) = crossbeam_channel::unbounded();
        let data = "test";
        let mock_stream = MockStream::new(data);
        let writer = create_writer(&log_path).expect("Failed to create writer");

        let mut ingestor = Ingestor::new(mock_stream, sender, Some(writer));

        ingestor.step();

        // drop ingestor to flush writer
        drop(ingestor);

        let file_contents = std::fs::read_to_string(&log_path).expect("Failed to read log file");

        assert_eq!(file_contents, data);
        assert_eq!(receiver.recv().unwrap(), data);
    }
    #[rstest::rstest]
    fn when_ingestor_reads_from_historical_file_then_output_is_the_same(
        test_path: TestPath,
        test_data_path: std::path::PathBuf,
    ) {
        let input_path = &test_data_path.join("test_ingestor_log.txt");
        let log_path = test_path.path.join("test_log.log");
        let (sender, _receiver) = crossbeam_channel::unbounded();
        let source = FileDataSource::new(input_path).expect("Failed to create data source");
        let writer = create_writer(&log_path).expect("Failed to create writer");

        let mut ingestor = Ingestor::new(source, sender, Some(writer));
        let mut cont = true;
        while cont {
            cont = ingestor.step();
        }

        // drop ingestor to flush writer
        drop(ingestor);

        let input_file_contents =
            std::fs::read_to_string(input_path).expect("Failed to read log file");
        let output_file_contents =
            std::fs::read_to_string(&log_path).expect("Failed to read log file");

        assert_eq!(output_file_contents, input_file_contents);
    }
}
