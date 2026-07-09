use std::convert::Infallible;
use std::time::SystemTime;

use crate::airspace::detail::AirspaceUpdate;
use crate::core::central_disk_logger::interface::IntoLogMessage;
use crate::pb::airspace::PbAirspaceUpdate;

impl IntoLogMessage<PbAirspaceUpdate> for AirspaceUpdate {
    type Error = Infallible;
    fn message_timestamp(&self) -> chrono::prelude::DateTime<chrono::prelude::Utc> {
        self.timestamp
    }
    fn into_message(self) -> Result<PbAirspaceUpdate, Self::Error> {
        let sys_time: SystemTime = self.timestamp.into();
        let timestamp = Some(sys_time.into());

        Ok(PbAirspaceUpdate {
            timestamp,
            updates: self.updates.into_iter().map(Into::into).collect(),
        })
    }
}

impl From<AirspaceUpdate> for PbAirspaceUpdate {
    fn from(airspace_update: AirspaceUpdate) -> Self {
        let sys_time: SystemTime = airspace_update.timestamp.into();
        let timestamp = Some(sys_time.into());

        Self {
            timestamp,
            updates: airspace_update
                .updates
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}
