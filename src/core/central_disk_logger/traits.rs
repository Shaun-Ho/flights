use crate::core::central_disk_logger::interface::LoggerHandle;

pub trait MessageLogger<T> {
    type Error;

    fn log(&self, message: T) -> Result<(), Self::Error>;
}
impl<T, M, E> MessageLogger<T> for LoggerHandle<M>
where
    T: TryInto<M, Error = E>,
    M: prost::Message,
{
    type Error = crate::core::central_disk_logger::errors::LoggingError<E>;

    fn log(&self, message: T) -> Result<(), Self::Error> {
        self.send(message)
    }
}

pub struct NoOpLogger;
impl<T> MessageLogger<T> for NoOpLogger {
    type Error = std::convert::Infallible;

    fn log(&self, _message: T) -> Result<(), Self::Error> {
        Ok(())
    }
}
