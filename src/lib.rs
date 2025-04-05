
mod common;
mod jobs;
mod types;

pub mod prelude {
    pub use crate::types::{Jobs, Job, JobData, JobID, JobTasks, Task, PGTask};
    pub use crate::jobs::{PGJobsPlugin, JobSettings, JobScheduler, JobSchedule,
        StopJobEvent, StartJobEvent, JobCatalog, JobPaused, TaskSets}; 

    pub use crate::common::*;

}

pub mod macros {
    pub use pg_jobs_macros::PGTask;
}