use bevy::app::{App, Plugin, PreUpdate, Update};
use bevy::ecs::schedule::common_conditions::on_event;
use bevy::ecs::schedule::{IntoSystemConfigs, Condition};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{Event, EventWriter, EventReader};
use bevy::ecs::system::{Resource, Res, ResMut};
use bevy::ecs::system::Commands;
use bevy::log::info;

use bevy_pg_calendar::prelude::{Calendar, CalendarNewHourEvent, Cron};
use serde::{Deserialize, Serialize};

use crate::pg_tasks::JobTasks;
pub struct PGJobsPlugin {
    pub active: bool 
}
impl Default for PGJobsPlugin {
    fn default() -> Self {
        PGJobsPlugin{
            active:        true
        }
    }
}

impl Plugin for PGJobsPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(Jobs::init(self.active))
        // .insert_resource(JobCatalog::init())
        .add_event::<TriggerJobEvent>()
        .add_event::<TriggerPreJobEvent>()
        .add_systems(PreUpdate, (trigger_jobs_calendar.run_if(on_event::<CalendarNewHourEvent>()), 
                                 trigger_jobs_time)
                                .chain()
                                .run_if(if_jobs_active))
        .add_systems(Update,    init_jobs.run_if(if_jobs_active.and_then(on_event::<TriggerJobEvent>())))

        // .add_systems(Update,    init_pre_jobs.run_if(on_event::<TriggerPrejob>()))
        // .add_systems(Update,    update_tasks.run_if(if_active)
        //                                     .after(init_jobs))
        // .add_systems(Update,    update_fail_jobs.run_if(if_active)
        //                                         .after(init_jobs))
        ;
    }
}

fn init_jobs(){
    info!("Init Jobs triggered");
}

#[derive(Event)]
pub struct TriggerJobEvent{
    pub entity: Entity
}

#[derive(Event)]
pub struct TriggerPreJobEvent;

pub fn if_jobs_active(jobs: Res<Jobs>) -> bool {
    jobs.active
}

// #[derive(Resource)]
// pub struct JobCatalog {
//     pub data: Vec<Job>
// }
// impl JobCatalog {
//     pub fn init() -> Self {
//         JobCatalog { data: Vec::new() }
//     }
//     pub fn add(&mut self, job: Job) {
//         self.data.push(job);
//     }
//     pub fn clear(&mut self){
//         self.data.clear();
//     } 
// }

#[derive(Resource)]
pub struct Jobs {
    active: bool,                   // For all jobs
    // data:   HashMap<Entity, Job>    // Map Option with Entity, but on the beginning of the job there might not be entity
    data:   Vec<Job>
}
impl Jobs {
    fn init(active: bool) -> Self {
        // Self {active, data: HashMap::new()}
        Self {active, data: Vec::new()}
    }
    pub fn upsert(&mut self, job: Job) {
        self.data.push(job); // This allows for multiple jobs per entity :o
    }
    // pub fn upsert(&mut self, entity: Entity, job: Job) {
    //     self.data.insert(entity, job);
    // }
    // pub fn remove(&mut self, entity: &Entity) {
    //     self.data.remove(entity);
    // }
    pub fn activate(&mut self) {
        self.active = true;
    }
    pub fn deactivate(&mut self) {
        self.active = false;
    }
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum JobStatus {
    ToDo,
    Active,
    Done
}

// #[derive(Clone)]
pub struct Job {
    pub entity:        Option<Entity>,    // In the beginning there might not be entity.
    pub status:        JobStatus,
    pub schedule:      JobSchedule,       // Schedule that will start the Job
    pub tasks:         JobTasks,          // List of job.set_active();tasks to be performed by entity
    pub fail_task_id:  u32,               // ID of task to perform if task failed
    pub fail_job_id:   u32,               // ID of task to perform if job failed to start 
    pub active:        bool,              // Toggle to activate/deactivate single task
    pub prejob:        bool,              // Flag if there needs to be a prejob
}

impl Default for Job {
    fn default() -> Self {
        Job{
            entity:         None,
            status:         JobStatus::ToDo,
            schedule:       JobSchedule::Instant, 
            tasks:          JobTasks::new(),
            fail_task_id:   0,
            fail_job_id:    0,
            active:         true,
            prejob:         false
        }
    }
}

impl Job {
    pub fn new() -> Self {

        let job = Job{
            entity:       None,
            status:       JobStatus::ToDo,
            tasks:        JobTasks::new(), 
            schedule:     JobSchedule::Instant,
            fail_job_id:  0,
            fail_task_id: 0,
            active:       true,
            prejob:       false
        };
        return job;
    }
    pub fn get_status(&mut self) -> JobStatus {
        self.status
    }
    pub fn set_active(&mut self){
        self.status = JobStatus::Active;
    }
    pub fn set_done(&mut self){
        self.status = JobStatus::Done;
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum JobSchedule {
    OnDemand,
    Instant,
    RealDelay(f32),      // Real time delay         
    Cron(Cron),
    Delay(u8)
         // Delay in in-game time hours - Should be one time . 
                    //Real time delay would require another trigger check function (current one is triggered only by change in in-game hour)
} 



// Update jobs. Triggers every hour from calendar.
fn trigger_jobs_calendar(mut commands:         Commands,
                         calendar:             Res<Calendar>,
                         mut jobs:             ResMut<Jobs>,
                         mut trigger_prejob:   EventWriter<TriggerPreJobEvent>,
                         mut trigger_job:      EventWriter<TriggerJobEvent>){


    for job in jobs.data.iter_mut(){

        if !job.active {
            continue;
        }

        if job.get_status() != JobStatus::ToDo {
            continue;
        }

        match &job.schedule {
            JobSchedule::Cron(cron) => {
                if cron.hours.as_ref().unwrap().contains(&calendar.get_current_hour()) && 
                   cron.days_week.as_ref().unwrap().contains(&calendar.get_current_weekday()){

                    let entity = job.tasks.start(&mut commands);
                        
                    if job.prejob {
                        trigger_prejob.send(TriggerPreJobEvent);
                    } else {
                        trigger_job.send(TriggerJobEvent{entity});
                    }
                    job.set_active();
                }
             }

            _=> {}
        }
    }

}

// Updates jobs on real time
fn trigger_jobs_time(mut commands:           Commands,
                     mut jobs:               ResMut<Jobs>,
                     mut trigger_prejob:     EventWriter<TriggerPreJobEvent>,
                     mut trigger_job:        EventWriter<TriggerJobEvent>,){

    for job in jobs.data.iter_mut(){

        if !job.active {
            continue;
        }
        if job.get_status() != JobStatus::ToDo {
            continue;
        }

        match &job.schedule {
            JobSchedule::Instant => {
                let entity = job.tasks.start(&mut commands);
                if job.prejob {
                    trigger_prejob.send(TriggerPreJobEvent);
                } else {
                    trigger_job.send(TriggerJobEvent{entity});
                }
                job.set_active();
            }
            _=> {}
        }
    }
}
