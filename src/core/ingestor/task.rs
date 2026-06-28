use std::net::ToSocketAddrs;

use prost::Message;

use crate::core::ingestor::config::GliderNetConfig;
use crate::core::ingestor::errors;
use crate::core::ingestor::protobuf::PbAprsPacket;
use crate::core::thread_manager::{SteppableTask, TaskState};

pub const INGESTOR_CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub struct Ingestor {
    source: Box<dyn APRSDataSource>,
    sender: crossbeam_channel::Sender<AprsPacket>,
}
impl Ingestor {
    pub fn new<C: APRSDataSource + 'static>(
        source: C,
        sender: crossbeam_channel::Sender<AprsPacket>,
    ) -> Self {
        Self {
            source: Box::new(source),
            sender,
        }
    }

    pub fn read_data_from_file(
        read_path: &std::path::Path,
        sender: crossbeam_channel::Sender<AprsPacket>,
    ) -> Result<Self, std::io::Error> {
        log::info!(
            "Reading APRS data from file: {}",
            read_path.to_string_lossy()
        );
        let source = ReplaySource::new(read_path)?;
        Ok(Self::new(source, sender))
    }

    pub fn connect_glidernet(
        config: &GliderNetConfig,
        sender: crossbeam_channel::Sender<AprsPacket>,
    ) -> Result<Self, std::io::Error> {
        log::info!("Connecting to TCP stream.");

        let address_str = format!("{}:{}", config.host, config.port);

        let mut resolved_address = address_str.to_socket_addrs()?;

        let address = resolved_address.next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Could not resolve host")
        })?;

        let mut stream =
            std::net::TcpStream::connect_timeout(&address, INGESTOR_CONNECTION_TIMEOUT)?;

        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));

        authentication_handshake(&mut stream, &config.filter)?;

        let source = LiveSource::new(stream);

        Ok(Self::new(source, sender))
    }
}

impl SteppableTask for Ingestor {
    fn step(&mut self) -> TaskState {
        match self.source.create_aprs_packet() {
            Ok(aprs_packet) => {
                if let Err(err) = self.sender.send(aprs_packet) {
                    log::error!("Ingestor: Failed to send to channel: {err}");
                    return TaskState::Running;
                }

                TaskState::Running
            }
            // This is to handle disconnected channels - only case where we should terminate the task
            Err(errors::PacketError::Disconnected) => {
                log::error!("Stream disconnected");
                TaskState::Completed
            }
            Err(err) => {
                log::error!("{err}");
                TaskState::Running
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AprsPacket {
    pub timestamp: std::time::SystemTime,
    pub message: bytes::Bytes,
}

pub trait APRSDataSource: Send {
    fn create_aprs_packet(&mut self) -> Result<AprsPacket, errors::PacketError>;
}

struct LiveSource<R: std::io::Read> {
    pub reader: std::io::BufReader<R>,
}
impl<R: std::io::Read> LiveSource<R> {
    pub fn new(tcp_stream: R) -> Self {
        Self {
            reader: std::io::BufReader::new(tcp_stream),
        }
    }
}
impl<R: std::io::Read + Send> APRSDataSource for LiveSource<R> {
    fn create_aprs_packet(&mut self) -> Result<AprsPacket, errors::PacketError> {
        let mut buffer = Vec::new();
        match std::io::BufRead::read_until(&mut self.reader, b'\n', &mut buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    log::info!("End of TCP stream");
                    return Err(errors::PacketError::Disconnected);
                }

                let timestamp = std::time::SystemTime::now();

                let packet = AprsPacket {
                    timestamp,
                    message: buffer.into(),
                };
                Ok(packet)
            }
            Err(error) => {
                let packet_error = errors::PacketError::IoError(error);
                log::error!("{packet_error}");
                Err(packet_error)
            }
        }
    }
}

struct ReplaySource {
    cursor: std::io::Cursor<Vec<u8>>,
    first_packet_timestamp: Option<std::time::SystemTime>,
    first_replay_time: Option<std::time::Instant>,
}
impl ReplaySource {
    pub fn new(input_path: &std::path::Path) -> Result<Self, std::io::Error> {
        let bytes = std::fs::read(input_path)?;
        let cursor = std::io::Cursor::new(bytes);
        Ok(Self {
            cursor,
            first_packet_timestamp: None,
            first_replay_time: None,
        })
    }
}
impl APRSDataSource for ReplaySource {
    fn create_aprs_packet(&mut self) -> Result<AprsPacket, errors::PacketError> {
        let position = usize::try_from(self.cursor.position())
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

        if position >= self.cursor.get_ref().len() {
            return Err(errors::PacketError::Disconnected);
        }

        match PbAprsPacket::decode_length_delimited(&mut self.cursor) {
            Ok(pb_aprs_packet) => {
                if let Some(packet_timestamp) = pb_aprs_packet.timestamp
                    && let Ok(packet_system_time) =
                        std::time::SystemTime::try_from(packet_timestamp)
                {
                    // set first_replay_time and first_packet_timestamp with the first packet
                    let start_time = *self
                        .first_replay_time
                        .get_or_insert_with(std::time::Instant::now);
                    let first_time = *self
                        .first_packet_timestamp
                        .get_or_insert(packet_system_time);

                    // Calculate the duration from the first packet to current packet
                    if let Ok(target_offset) = packet_system_time.duration_since(first_time) {
                        let elapsed = start_time.elapsed();

                        if target_offset > elapsed {
                            std::thread::sleep(target_offset.checked_sub(elapsed).unwrap());
                        }
                    }
                }
                pb_aprs_packet
                    .try_into()
                    .map_err(errors::PacketError::Conversion)
            }
            Err(error) => {
                let decode_error = errors::PacketError::DecodeReadError(error);
                log::error!("{decode_error}");
                Err(decode_error)
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

#[cfg(test)]
mod test {
    use std::io::Write;

    use rstest;

    use crate::core::ingestor::task::{APRSDataSource, Ingestor, LiveSource, ReplaySource};
    use crate::core::ingestor::task::{AprsPacket, PbAprsPacket};
    use crate::core::thread_manager::{SteppableTask, TaskState};
    use crate::test_utilities::{TestPath, test_path, write_pb_message_to_disk};

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

    struct MockStatefulStream {
        state: usize,
    }

    impl std::io::Read for MockStatefulStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.state {
                0 => {
                    self.state = 1;
                    let payload = b"PACKET_1\n";
                    buf[..payload.len()].copy_from_slice(payload);
                    Ok(payload.len())
                }
                1 => {
                    self.state = 2;
                    Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "simulated network silence",
                    ))
                }
                2 => {
                    self.state = 3;
                    let payload = b"PACKET_2\n";
                    buf[..payload.len()].copy_from_slice(payload);
                    Ok(payload.len())
                }
                _ => Ok(0), // EOF (Disconnected)
            }
        }
    }

    fn create_writer(
        output_path: &std::path::Path,
    ) -> Result<std::io::BufWriter<std::fs::File>, std::io::Error> {
        std::fs::File::create_new(output_path).map(std::io::BufWriter::new)
    }
    #[test]
    fn given_connection_to_stream_when_data_received_then_ingestor_sends_correct_data_and_keeps_running()
     {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let data = "APRS_PACKET_DATA\n";
        let mock_stream = MockStream::new(data);
        let source = LiveSource::new(mock_stream);

        let mut ingestor = Ingestor::new(source, sender);

        let keep_running = ingestor.step();

        assert!(matches!(keep_running, TaskState::Running));
        assert_eq!(receiver.recv().unwrap().message, data);
    }

    #[test]
    fn given_stream_when_idle_timeout_occurs_then_ingestor_keeps_running_and_reads_next_packet() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let mock_stream = MockStatefulStream { state: 0 };
        let source = LiveSource::new(mock_stream);
        let mut ingestor = Ingestor::new(source, sender);

        let keep_running = ingestor.step();
        // Expected to keep running after valid packet
        assert!(matches!(keep_running, TaskState::Running));
        assert_eq!(receiver.recv().unwrap().message, "PACKET_1\n");

        let keep_running = ingestor.step();
        // Expected to keep running after idle timeout
        assert!(matches!(keep_running, TaskState::Running));
        assert!(
            receiver.try_recv().is_err(),
            "No data should be sent on timeout"
        );

        let keep_running = ingestor.step();
        // Expected to keep running after recovering valid packet
        assert!(matches!(keep_running, TaskState::Running));
        assert_eq!(receiver.recv().unwrap().message, "PACKET_2\n");

        let keep_running = ingestor.step();
        // Expected to stop task when TCP stream is disconnected.
        assert!(matches!(keep_running, TaskState::Completed));
    }
    #[test]
    fn given_connection_to_stream_when_end_of_stream_then_ingestor_stops_running() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let data = "";
        let mock_stream = MockStream::new(data);
        let source = LiveSource::new(mock_stream);
        let mut ingestor = Ingestor::new(source, sender);

        let keep_running = ingestor.step();

        assert!(matches!(keep_running, TaskState::Completed));

        assert!(receiver.try_recv().is_err(), "Channel should be empty");
    }

    #[rstest::rstest]
    #[test_log::test]
    fn when_ingestor_reads_from_log_file_then_sender_receives_expected_aprs_packet(
        test_path: TestPath,
    ) {
        let log_path = &test_path.path.join("test_ingestor_log.pb");

        let now = std::time::SystemTime::now();
        let timestamp = prost_types::Timestamp::from(now);
        let expected_aprs_packet = PbAprsPacket {
            timestamp: Some(timestamp),
            message: "aprs\n".into(),
        };

        {
            // explicitly flush writer and drop within closure
            let mut writer = create_writer(log_path).expect("Failed to create writer");
            let _ = write_pb_message_to_disk(&mut writer, &expected_aprs_packet);
            writer.flush().unwrap();
        }

        let (sender, receiver) = crossbeam_channel::unbounded();
        let source = ReplaySource::new(log_path).expect("Failed to create data source");

        let mut ingestor = Ingestor::new(source, sender);
        let mut cont = true;
        while cont {
            cont = matches!(ingestor.step(), TaskState::Running);
        }

        // drop ingestor to flush writer
        drop(ingestor);

        let vec: Vec<AprsPacket> = receiver.iter().collect();
        assert!(vec.len() == 1);
        assert_eq!(
            *vec.first().unwrap(),
            expected_aprs_packet.try_into().unwrap()
        );
    }

    #[rstest::rstest]
    fn when_reading_from_replay_source_then_delays_are_applied_correctly(test_path: TestPath) {
        let log_path = test_path.path.join("test_replay_delay.pb");

        let base_time = std::time::SystemTime::now();
        let time_p1 = base_time;
        let time_p2 = base_time + std::time::Duration::from_millis(50);
        let time_p3 = base_time + std::time::Duration::from_millis(100);

        let packet1 = PbAprsPacket {
            timestamp: Some(prost_types::Timestamp::from(time_p1)),
            message: "packet 1\n".into(),
        };
        let packet2 = PbAprsPacket {
            timestamp: Some(prost_types::Timestamp::from(time_p2)),
            message: "packet 2\n".into(),
        };
        let packet3 = PbAprsPacket {
            timestamp: Some(prost_types::Timestamp::from(time_p3)),
            message: "packet 3\n".into(),
        };

        // Write the packets to the mock log file
        {
            let mut writer = create_writer(&log_path).expect("Failed to create writer");
            write_pb_message_to_disk(&mut writer, &packet1).unwrap();
            write_pb_message_to_disk(&mut writer, &packet2).unwrap();
            write_pb_message_to_disk(&mut writer, &packet3).unwrap();
            writer.flush().unwrap();
        }

        let mut source = ReplaySource::new(&log_path).expect("Failed to open replay source");

        let start = std::time::Instant::now();

        let p1 = source.create_aprs_packet().unwrap();
        assert_eq!(p1.message, "packet 1\n");
        let elapsed_p1 = start.elapsed().as_millis();
        assert!(elapsed_p1.abs_diff(0) <= 5);

        let p2 = source.create_aprs_packet().unwrap();
        assert_eq!(p2.message, "packet 2\n");
        let elapsed_p2 = start.elapsed().as_millis();
        assert!(elapsed_p2 >= 50);
        assert!(elapsed_p2.abs_diff(50) <= 5);

        let p3 = source.create_aprs_packet().unwrap();
        assert_eq!(p3.message, "packet 3\n");
        let elapsed_p3 = start.elapsed().as_millis();
        assert!(elapsed_p3 >= 100);
        assert!(elapsed_p3.abs_diff(100) <= 5);

        let p4 = source.create_aprs_packet().is_err();
        assert!(p4);
    }
}
