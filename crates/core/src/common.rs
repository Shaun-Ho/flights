pub struct JsonFormat;
pub struct ProtoFormat;

pub const MCAP_FILE_SUFFIX: &str = "mcap";

pub trait LogFormat {
    const MESSAGE_ENCODING: &'static str;
    const SCHEMA_ENCODING: &'static str;
}

impl LogFormat for JsonFormat {
    const MESSAGE_ENCODING: &'static str = "json";
    const SCHEMA_ENCODING: &'static str = "jsonschema";
}
impl LogFormat for ProtoFormat {
    const MESSAGE_ENCODING: &'static str = "protobuf";
    const SCHEMA_ENCODING: &'static str = "protobuf";
}
