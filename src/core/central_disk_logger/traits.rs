use crate::core::central_disk_logger::task::LoggerHandle;

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
