use crate::core::central_disk_logger::errors::LoggingError;
use crate::core::central_disk_logger::interface::LoggerHandle;

pub trait LogToDisk<T> {
    type Error;

    fn log(&self, message: T) -> Result<(), Self::Error>;
}
impl<T, M, E> LogToDisk<T> for LoggerHandle<M>
where
    T: TryInto<M, Error = E>,
    M: prost::Message,
{
    type Error = LoggingError<E>;

    fn log(&self, message: T) -> Result<(), Self::Error> {
        self.send(message)
    }
}
