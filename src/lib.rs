use bevy::prelude::*;

// mod pg_jobs;
// mod pg_tasks;
// mod tasks;
mod types;
// mod utils;

pub use types::{Job, JobData, JobID, JobTasks, Task, PGTask};


// pub use pg_tasks::{TasksPlugin, JobTasks};
// pub use pg_jobs::{PGJobsPlugin, Jobs, JobSettings, JobScheduler, 
//     StopJobEvent, StartJobEvent, JobData};  

// pub struct PGJobsPlugin;

// impl Plugin for AIPlugin {
//     fn build(&self, app: &mut App) {
//         app 
//         ;
//     }
// }


pub mod prelude {
    pub use crate::types::{Job, JobData, JobID, JobTasks, Task, PGTask};
}
