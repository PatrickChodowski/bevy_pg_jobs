
mod jobs;
mod types;

pub mod prelude {
    pub use crate::types::{Job, JobData, JobID, JobTasks, Task, PGTask};
    pub use crate::jobs::{PGJobsPlugin, JobSettings, JobScheduler, 
        StopJobEvent, StartJobEvent, JobCatalog, JobPaused, TaskSets}; 
}
