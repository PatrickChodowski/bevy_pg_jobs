
#[cfg(feature="common")]
mod common;

mod jobs;
mod types;

pub mod prelude {
    pub use crate::types::{Job, JobData, JobID, JobTasks, Task, PGTask, JobOnFail};
    pub use crate::jobs::{PGJobsPlugin, JobSettings, JobScheduler, JobSchedule,
        StopJobEvent, StartJobEvent, JobCatalog, JobPaused, TaskSets, PGJobsSet, if_jobs_active}; 

    #[cfg(feature="common")]
    pub use crate::common::*;

}

pub mod macros {
    pub use pg_jobs_macros::PGTask;
}