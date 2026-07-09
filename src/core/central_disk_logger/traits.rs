use chrono::{DateTime, Utc};

pub trait LogSender<M> {
    type Error;
    fn send(&self, message: M) -> Result<(), Self::Error>;
}

pub trait IntoLogMessage<M> {
    type Error;
    fn into_message(self) -> Result<M, Self::Error>;
    fn message_timestamp(&self) -> DateTime<Utc>;
}
