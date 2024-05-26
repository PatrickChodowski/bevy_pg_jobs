mod pg_jobs;
mod pg_tasks;


pub mod prelude {
    pub use crate::pg_tasks::{JobTasks, TaskType, TaskStatus, TasksPlugin};
    pub use crate::pg_jobs::{PGJobsPlugin, Jobs, JobSchedule, Job};  
}

pub mod tasks {
    pub use crate::pg_tasks::{TaskType, SpawnTask, DespawnTask, RotateTask, MoveTask, WaitTask};
}