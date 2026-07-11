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

pub trait McapSchemaDescriptor {
    fn schema_name() -> String;
    fn encoding() -> &'static str;
    fn schema_bytes() -> Vec<u8>;
    fn topic() -> String;
}
