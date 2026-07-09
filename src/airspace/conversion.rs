use std::time::SystemTime;

use crate::airspace::detail::AirspaceUpdate;
use crate::pb::airspace::PbAirspaceUpdate;

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
