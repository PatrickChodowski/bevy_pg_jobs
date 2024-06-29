use bevy::prelude::*;

mod pg_jobs;
mod pg_tasks;
mod tasks;
mod utils;

pub use pg_tasks::TasksPlugin;
pub use pg_jobs::{PGJobsPlugin, Jobs, JobSettings, JobScheduler, StopJobEvent, StartJobEvent};  

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app 
        .add_plugins(PGJobsPlugin::default())
        .add_plugins(TasksPlugin) 
        ;
    }
}
