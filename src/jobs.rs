use bevy::prelude::*;
use bevy::app::{App, Plugin, PreUpdate, Update, Startup};
use bevy::asset::{Asset, AssetServer, Assets, LoadedFolder, Handle};
use bevy::ecs::schedule::common_conditions::on_message;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::entity::Entity;
use bevy::ecs::message::{Message, MessageReader};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::component::Component;
use bevy::ecs::system::{Commands, Local, Res, ResMut};
use bevy::ecs::resource::Resource;
use bevy::reflect::{Reflect, TypePath};
// use bevy_common_assets::json::JsonAssetPlugin;
// use bevy_common_assets::toml::TomlAssetPlugin;
use bevy_pg_calendar::prelude::{Calendar, CalendarNewHourEvent, Cron};
use std::hash::Hash;

use super::types::{PGTask, JobData, Job};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TaskSets {
    Dispatch,
    Extension,
    Simple,
    Loop,
    Decision
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct PGJobsSet;


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
        // .register_type_data::<Box<dyn PGTask>, ReflectSerialize>()
        // .register_type_data::<Box<dyn PGTask>, ReflectDeserialize>()

        .add_message::<StopJobEvent>()
        .add_message::<StartJobEvent>()

        .configure_sets(Update, PGJobsSet.run_if(if_jobs_active))
        .configure_sets(
            Update, (
                TaskSets::Dispatch, 
                TaskSets::Extension,
                TaskSets::Decision, 
                TaskSets::Simple, 
                TaskSets::Loop
            ).chain().in_set(PGJobsSet)
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

        .add_systems(Startup,   init)
        .add_systems(Update,    track.run_if(resource_exists::<LoadedJobDataHandles>
                                     .and(resource_exists::<LoadedJobTriggerHandles>)))

        .add_systems(PreUpdate, (
                trigger_jobs_calendar.run_if(on_message::<CalendarNewHourEvent>), 
                trigger_jobs_time
            ).chain().run_if(if_jobs_active)
        )

        .add_systems(PreUpdate, (
                stop_job.run_if(on_message::<StopJobEvent>), 
                start_job.run_if(on_message::<StartJobEvent>)
            ).chain()
        );

        #[cfg(feature="verbose")]
        app.add_observer(observe_add_job);

        #[cfg(feature="verbose")]
        app.add_observer(observe_replace_job);

        #[cfg(feature="verbose")]
        app.add_observer(observe_remove_job);
    }
}

#[cfg(feature="verbose")]
fn observe_add_job(
    trigger: Trigger<OnAdd, Job>,
    jobs: Query<&Job>
){
    info!("Added Job {} to {}", jobs.get(trigger.target()).unwrap().name, trigger.target());
}

#[cfg(feature="verbose")]
fn observe_replace_job(
    trigger: Trigger<OnReplace, Job>,
    jobs: Query<&Job>
){
    info!("Replaced old Job to {} on {}",  jobs.get(trigger.target()).unwrap().name, trigger.target());
}

#[cfg(feature="verbose")]
fn observe_remove_job(
    trigger: Trigger<OnRemove, Job>,
    jobs: Query<&Job>
){
    info!("Removed Job {} from {}", jobs.get(trigger.target()).unwrap().name, trigger.target());
}




#[derive(Message)]
pub struct StopJobEvent {
    pub entity:     Entity
}

#[derive(Message)]
pub struct StartJobEvent {
    pub name: &'static str,
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
    // let handle_folder_jobdata: Handle<LoadedFolder> = ass.load_folder("jobs/data");
    // let handle_folder_jobtrigger: Handle<LoadedFolder> = ass.load_folder("jobs/triggers");
    // commands.insert_resource(LoadedJobDataHandles(handle_folder_jobdata));
    // commands.insert_resource(LoadedJobTriggerHandles(handle_folder_jobtrigger));
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
        if let Some(jobdata_load_state) = ass.get_recursive_dependency_load_state(&loaded_jobdata.0) {
            if jobdata_load_state.is_loaded(){
                job_ready.data_ready = true;
            }
        }
    }
    if !job_ready.triggers_ready {
        if let Some(jobtriggers_load_state) = ass.get_recursive_dependency_load_state(&loaded_jobtrigger.0) {
            if jobtriggers_load_state.is_loaded(){
                job_ready.triggers_ready = true;
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
                info!(" [JOBS] Added JobTrigger {} active: {}", 
                     jobtrigger.trigger_id, jobtrigger.active);
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

// Stores JobDatas from assets job.toml files
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

    pub fn get(&self, job_name: &'static str) -> Option<&JobData> {
        for jd in self.data.iter(){
            if jd.name == job_name {
                return Some(jd);
            }
        }
        return None;
    }

    pub fn assign(
        &self, 
        commands:   &mut Commands, 
        entity:     Entity,
        job_name:   &'static str, 
    ){
        commands.entity(entity).remove::<Job>();
        if let Some(jobdata) = self.get(job_name){
            let mut job = Job::new(jobdata.clone());
            job.set_active();
            commands.entity(entity).insert(job);
            if let Some(first_task) = jobdata.tasks.get_current(){
                first_task.task.insert(commands, &entity);
            } else {
                error!("Could not start first task for entity: {}", entity);
            }
        } else {
            error!("Could not assign job: {} to entity: {}", job_name, entity);
        }

    }

    pub fn start(
        &self, 
        commands: &mut Commands, 
        job_name:   &'static str
    ) -> Option<Entity> {
        if let Some(jobdata) = self.get(job_name){
            if let Some(job_entity) = jobdata.start(commands){
                return Some(job_entity);
            }
        }
        return None;
    }
}

/// Settings for all jobs
#[derive(Resource)]
pub struct JobSettings {
    active: bool,
    debug:  bool
}
impl JobSettings {
    fn init(
        active: bool, 
        debug: bool
    ) -> Self {
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
    pub fn get(&self, trigger_id: u32) -> Option<JobTrigger> {
        for jobtrigger in self.data.iter() {
            if jobtrigger.trigger_id == trigger_id {
                return Some(jobtrigger.clone());
            }
        }
        error!("Missing job trigger id in the scheduler: {}", trigger_id);
        return None;
    }
    pub fn deactivate_all(&mut self){
        info!(" [JOBS DEBUG] Deactivate all triggers");
        for jobtrigger in self.data.iter_mut(){
            jobtrigger.active = false;
        }
    }
    pub fn activate_all(&mut self){
        info!(" [JOBS DEBUG] Activate all triggers");
        for jobtrigger in self.data.iter_mut(){
            jobtrigger.active = true;
        }
    }
    pub fn activate(&mut self, trigger_id: &u32){
        info!(" [JOBS DEBUG] Activate trigger: {}", trigger_id);
        for jobtrigger in self.data.iter_mut(){
            if &jobtrigger.trigger_id == trigger_id {
                jobtrigger.active = true;
                break;
            }
        }
    }
    pub fn deactivate(&mut self, trigger_id: &u32){
        info!(" [JOBS DEBUG] Deactivate trigger: {}", trigger_id);
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
    pub name:          &'static str,
    pub trigger_id:    u32,
    pub schedule:      JobSchedule,
    pub active:        bool
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum JobSchedule {      
    Instant,             // Start instantly       
    Cron(Cron),          // Waiting for Cron 
    Delay(u8),           // Delay in in-game hours
    RealDelay(f32)       // Real time delay  
} 
impl JobSchedule {
    pub fn parse(&mut self) {
        match self {
            JobSchedule::Cron(cron) => {cron.parse()}
            _ => {}
        }
    }
}

// Update jobs. Triggers every hour from calendar.
fn trigger_jobs_calendar(
    mut commands:         Commands,
    calendar:             Res<Calendar>,
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
                    job_catalog.start(&mut commands, job_trigger.name);
                }
             }
            _=> {}
        }
    }

}

// Updates jobs on real time
fn trigger_jobs_time(
    mut commands:         Commands,
    job_scheduler:        Res<JobScheduler>,
    job_catalog:          Res<JobCatalog>
){
    for job_trigger in job_scheduler.data.iter(){

        if !job_trigger.active {
            continue;
        }

        match &job_trigger.schedule {
            JobSchedule::Instant => {
                job_catalog.start(&mut commands, job_trigger.name);
            }
            _=> {}
        }
    }
}


fn stop_job(
    mut commands:       Commands,
    mut stop_job:       MessageReader<StopJobEvent>
){
    for ev in stop_job.read(){
        #[cfg(feature="verbose")]
        info!(" [JOBS] Removing job from entity: {:?}", ev.entity);
        commands.entity(ev.entity).remove::<Job>();
    }

}

fn start_job(
    mut commands:       Commands,
    jobs_catalog:       Res<JobCatalog>,
    mut start_job:      MessageReader<StartJobEvent>
){
    for ev in start_job.read(){
        #[cfg(feature="verbose")]
        info!(" [JOBS] Adding job {} to entity {:?}", ev.name, ev.entity);

        jobs_catalog.assign(&mut commands, ev.entity, ev.name);
    }
}


#[derive(Component, Reflect)]
#[reflect(Component)]
struct JobDebug;
