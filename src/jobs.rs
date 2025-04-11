use bevy::prelude::*;
use bevy::app::{App, Plugin, PreUpdate, Update, Startup};
use bevy::asset::{Asset, AssetServer, Assets, LoadedFolder, Handle};
use bevy::ecs::schedule::common_conditions::on_event;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{Event, EventReader};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::component::Component;
use bevy::ecs::system::{Commands, Local, Res, ResMut, Query};
use bevy::ecs::resource::Resource;
use bevy::ecs::query::With;
use bevy::reflect::{Reflect, TypePath};
// use bevy_common_assets::json::JsonAssetPlugin;
// use bevy_common_assets::toml::TomlAssetPlugin;
use bevy_pg_calendar::prelude::{Calendar, CalendarNewHourEvent, Cron};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::common::{
    DespawnTask, DespawnWithDelay, LoopTask, HideTask, ShowTask, TeleportTask, WaitTask, RandomWaitTask
};
use super::types::{JobTasks, JobData, Jobs, Job, JobID};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TaskSets {
    Dispatch,
    Extension,
    Simple,
    Loop,
    Decision
}

pub struct PGJobsPlugin {
    pub active: bool,
    pub debug:  bool
}
impl Default for PGJobsPlugin {
    fn default() -> Self {
        PGJobsPlugin{
            active:        true,
            debug:         true
        }
    }
}

impl Plugin for PGJobsPlugin {
    fn build(&self, app: &mut App) {
        app
        .register_type::<Jobs>()
        // .register_type_data::<Jobs, ReflectSerialize>()
        .register_type::<Job>()
        .register_type::<JobID>()
        .register_type::<JobData>()

        .register_type::<JobTasks>()
        // .register_type_data::<HashMap<u32, Task>, ReflectSerialize>()
        // .register_type_data::<HashMap<u32, Task>, ReflectDeserialize>()

        .register_type::<JobStatus>()
        .register_type::<JobSchedule>()
        .register_type::<JobDebug>()
        .register_type::<JobPaused>()

        // .register_type_data::<Box<dyn PGTask>, ReflectSerialize>()
        // .register_type_data::<Box<dyn PGTask>, ReflectDeserialize>()

        .register_type::<DespawnTask>()
        .register_type::<DespawnWithDelay>()
        .register_type::<HideTask>()
        .register_type::<ShowTask>()
        .register_type::<RandomWaitTask>()
        .register_type::<WaitTask>()
        .register_type::<LoopTask>()
        .register_type::<TeleportTask>()

        .configure_sets(
            Update, (
                TaskSets::Dispatch, 
                TaskSets::Extension, 
                TaskSets::Simple, 
                TaskSets::Decision,
                TaskSets::Loop
            ).chain()
        )

        // .add_plugins(JsonAssetPlugin::<JobData>::new(&["job.json"]))
        // .add_plugins(TomlAssetPlugin::<JobData>::new(&["job.toml"]))
        // .add_plugins(JsonAssetPlugin::<JobTrigger>::new(&["trigger.json"]))
        // .add_plugins(TomlAssetPlugin::<JobTrigger>::new(&["trigger.toml"]))
        // .add_plugins(JsonAssetPlugin::<JobTriggers>::new(&["triggers.json"]))
        // .add_plugins(TomlAssetPlugin::<JobTriggers>::new(&["triggers.toml"]))

        .insert_resource(JobSettings::init(self.active, self.debug))
        .insert_resource(JobCatalog::init())
        .insert_resource(JobScheduler::init())
        .insert_resource(Jobs::init())

        .add_event::<StopJobEvent>()
        .add_event::<StartJobEvent>()

        .add_systems(Startup,   init)
        // .add_systems(Update,    track.run_if(resource_exists::<LoadedJobDataHandles>
        //                              .and(resource_exists::<LoadedJobTriggerHandles>)))

        .add_systems(PreUpdate, (
                trigger_jobs_calendar.run_if(on_event::<CalendarNewHourEvent>), 
                trigger_jobs_time
            ).chain().run_if(if_jobs_active)
        )

        .add_systems(PreUpdate, (
                stop_job.run_if(on_event::<StopJobEvent>), 
                start_job.run_if(on_event::<StartJobEvent>)
            ).chain()
        )
        ;
    }
}


#[derive(Event)]
pub struct StopJobEvent {
    pub entity: Entity
}

#[derive(Event)]
pub struct StartJobEvent {
    pub job_id: JobID,
    pub entity: Entity
}

#[derive(Resource)]
struct LoadedJobDataHandles(Handle<LoadedFolder>);

#[derive(Resource)]
struct LoadedJobTriggerHandles(Handle<LoadedFolder>);


// Read in all jobs from data files into asset server

fn init(
    mut commands:   Commands,
    ass:            Res<AssetServer>
){
    let handle_folder_jobdata: Handle<LoadedFolder> = ass.load_folder("jobs/data");
    let handle_folder_jobtrigger: Handle<LoadedFolder> = ass.load_folder("jobs/triggers");
    commands.insert_resource(LoadedJobDataHandles(handle_folder_jobdata));
    commands.insert_resource(LoadedJobTriggerHandles(handle_folder_jobtrigger));
}

struct JobsReady {
    data_ready: bool,
    triggers_ready: bool
}

impl Default for JobsReady {
    fn default() -> Self {
        JobsReady{ 
            data_ready: false,
            triggers_ready: false
        }
    }
}

fn track(
    mut commands:           Commands,
    ass:                    Res<AssetServer>,
    mut job_ready:          Local<JobsReady>, 
    mut ass_jobdata:        ResMut<Assets<JobData>>,
    mut ass_jobtrigger:     ResMut<Assets<JobTrigger>>,
    mut ass_triggers:       ResMut<Assets<JobTriggers>>,
    mut jobs_catalog:       ResMut<JobCatalog>,
    mut jobs_scheduler:     ResMut<JobScheduler>,
    loaded_jobdata:         Res<LoadedJobDataHandles>,
    loaded_jobtrigger:      Res<LoadedJobTriggerHandles>
){

    if !job_ready.data_ready {
        if let Some(scenes_load_state) = ass.get_recursive_dependency_load_state(&loaded_jobdata.0) {
            if scenes_load_state.is_loaded(){
                job_ready.data_ready = true;
            }
        }
    }
    if !job_ready.triggers_ready {
        if let Some(scenes_load_state) = ass.get_recursive_dependency_load_state(&loaded_jobtrigger.0) {
            if scenes_load_state.is_loaded(){
                job_ready.data_ready = true;
            }
        }
    }

    if job_ready.data_ready && job_ready.triggers_ready {

        for (_job_id, jobdata) in ass_jobdata.iter_mut(){
            jobs_catalog.add(jobdata.clone());
        }

        for (_job_id, jobtrigger) in ass_jobtrigger.iter_mut(){
            jobtrigger.schedule.parse();
            jobs_scheduler.add(jobtrigger.clone());
        }

        for (_job_id, trigger_data) in ass_triggers.iter_mut(){
            for jobtrigger in trigger_data.data.iter_mut(){
                jobtrigger.schedule.parse();
                jobs_scheduler.add(jobtrigger.clone());
            }
        }

        commands.remove_resource::<LoadedJobDataHandles>();
        commands.remove_resource::<LoadedJobTriggerHandles>();

    }

}

pub fn if_jobs_active(
    job_settings: Res<JobSettings>
) -> bool {
    job_settings.active
}

pub fn if_jobs_debug(
    job_settings: Res<JobSettings>
) -> bool {
    job_settings.debug
}


#[derive(Resource)]
pub struct JobCatalog {
    pub data: Vec<JobData>
}
impl JobCatalog {
    pub fn init() -> Self {
        JobCatalog { data: Vec::new() }
    }
    pub fn add(&mut self, jobdata: JobData) {
        self.data.push(jobdata);
    }
    pub fn clear(&mut self){
        self.data.clear();
    } 
    pub fn get(&self, id: JobID) -> JobData {
        for job in self.data.iter() {
            if job.id == id {
                return job.clone();
            }
        }
        panic!("Missing job id in the catalog: {}", id);
    }
    pub fn assign(&self, commands: &mut Commands, job_id: JobID, entity: Entity, jobs: &mut ResMut<Jobs>){
        jobs.remove_all_clean(commands, &entity);
        let jobdata = self.get(job_id);
        let mut job = Job::new(entity, jobdata.clone());
        job.set_active();
        jobs.add(job);
        let first_task = jobdata.tasks.get_current();
        first_task.task.insert_task(commands, &entity);
    }
    pub fn start(&self, commands: &mut Commands, job_id: JobID, jobs: &mut ResMut<Jobs>) -> Entity {
        let jobdata = self.get(job_id);
        let job_entity = jobdata.start(commands, jobs);
        return job_entity;
    }
}

/// Settings for all jobs
#[derive(Resource)]
pub struct JobSettings {
    active: bool,
    debug:  bool
}
impl JobSettings {
    fn init(active: bool, debug: bool) -> Self {
        Self {active, debug}
    }
    pub fn activate(&mut self) {
        self.active = true;
    }
    pub fn deactivate(&mut self) {
        self.active = false;
    }
    pub fn set_debug(&mut self, b: bool) {
        self.debug = b
    }
    pub fn get_debug(&self) -> bool {
        self.debug
    }
}

#[derive(Resource)]
pub struct JobScheduler {
    pub data: Vec<JobTrigger>
}
impl JobScheduler {
    fn init() -> Self {
        JobScheduler { data: Vec::new() }
    }
    pub fn add(&mut self, jobtrigger: JobTrigger) {
        self.data.push(jobtrigger);
    }
    pub fn clear(&mut self){
        self.data.clear();
    } 
    pub fn get(&self, trigger_id: u32) -> JobTrigger {
        for jobtrigger in self.data.iter() {
            if jobtrigger.trigger_id == trigger_id {
                return jobtrigger.clone();
            }
        }
        panic!("Missing job trigger id in the scheduler: {}", trigger_id);
    }
    pub fn deactivate_all(&mut self){
        for jobtrigger in self.data.iter_mut(){
            jobtrigger.active = false;
        }
    }
    pub fn activate_all(&mut self){
        for jobtrigger in self.data.iter_mut(){
            jobtrigger.active = true;
        }
    }
    pub fn activate(&mut self, trigger_id: &u32){
        for jobtrigger in self.data.iter_mut(){
            if &jobtrigger.trigger_id == trigger_id {
                jobtrigger.active = true;
                break;
            }
        }
    }
    #[allow(dead_code)]
    pub fn deactivate(&mut self, trigger_id: &u32){
        for jobtrigger in self.data.iter_mut(){
            if &jobtrigger.trigger_id == trigger_id {
                jobtrigger.active = false;
                break;
            }
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct JobPaused;


#[derive(PartialEq, Copy, Clone, Debug, Reflect)]
pub enum JobStatus {
    ToDo,
    Active,
    Done,
    Paused,
    Inactive
}

#[derive(Asset, TypePath, Clone, Debug)]
pub struct JobTriggers {
    pub data: Vec<JobTrigger>
}

#[derive(Asset, TypePath, Clone, Debug)]
pub struct JobTrigger {
    pub trigger_id:    u32,
    pub job_id:        JobID,
    pub schedule:      JobSchedule,
    pub active:        bool
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Reflect)]
pub enum JobSchedule {      
    Instant,             // Start instantly       
    Cron(Cron),          // Waiting for Cron 
    Delay(u8),           // Delay in in-game hours
    RealDelay(f32)       // Real time delay  
} 
impl JobSchedule {
    pub fn parse(&mut self) {
        match self {
            JobSchedule::Cron(ref mut cron) => {cron.parse()}
            _ => {}
        }
    }
}

// Update jobs. Triggers every hour from calendar.
fn trigger_jobs_calendar(
    mut commands:         Commands,
    calendar:             Res<Calendar>,
    mut jobs:             ResMut<Jobs>,
    job_scheduler:        Res<JobScheduler>,
    job_catalog:          Res<JobCatalog>
){

    for job_trigger in job_scheduler.data.iter(){

        if !job_trigger.active {
            continue;
        }

        match &job_trigger.schedule {
            JobSchedule::Cron(cron) => {
                if cron.is_time(&calendar){
                    job_catalog.start(&mut commands, job_trigger.job_id, &mut jobs);
                }
             }
            _=> {}
        }
    }

}

// Updates jobs on real time
fn trigger_jobs_time(
    mut commands:         Commands,
    mut jobs:             ResMut<Jobs>,
    job_scheduler:        Res<JobScheduler>,
    job_catalog:          Res<JobCatalog>
){
    for job_trigger in job_scheduler.data.iter(){

        if !job_trigger.active {
            continue;
        }

        match &job_trigger.schedule {
            JobSchedule::Instant => {
                job_catalog.start(&mut commands, job_trigger.job_id, &mut jobs);
            }
            _=> {}
        }
    }
}


fn stop_job(
    mut commands:       Commands,
    mut jobs:           ResMut<Jobs>,
    mut stop_job:       EventReader<StopJobEvent>,
    jobdebugs:          Query<(Entity, &ChildOf), With<JobDebug>>
){
    for ev in stop_job.read(){
        info!(" [JOBS] Removing all jobs for entity: {:?}", ev.entity);
        jobs.remove_all_clean(&mut commands, &ev.entity);

        for (text_entity, task_entity) in jobdebugs.iter(){
            if task_entity.parent == ev.entity {
                commands.entity(text_entity).despawn();
                break;
            }
        }
    }

}

fn start_job(
    mut commands:       Commands,
    mut jobs:           ResMut<Jobs>,
    jobs_catalog:       Res<JobCatalog>,
    mut start_job:      EventReader<StartJobEvent>
){
    for ev in start_job.read(){
        info!(" [JOBS] Adding job {} to entity {:?}", ev.job_id, ev.entity);
        jobs_catalog.assign(&mut commands, ev.job_id, ev.entity, &mut jobs);
    }
}


#[derive(Component, Reflect)]
#[reflect(Component)]
struct JobDebug;
