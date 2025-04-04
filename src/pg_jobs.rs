use bevy::prelude::*;
use bevy::app::{App, Plugin, PreUpdate, Update, Startup};
use bevy::asset::{Asset, AssetServer, Assets, LoadedFolder, Handle};
use bevy::ecs::schedule::common_conditions::{on_event, resource_exists};
use bevy::ecs::schedule::{IntoScheduleConfigs, Condition};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{Event, EventReader};
use bevy::ecs::reflect::{ReflectResource, ReflectComponent};
use bevy::ecs::component::Component;
use bevy::ecs::system::{Commands, Local, Res, ResMut, Query};
use bevy::ecs::resource::Resource;
use bevy::ecs::query::With;
use bevy::reflect::{TypePath, Reflect};
use bevy_common_assets::json::JsonAssetPlugin;
use bevy_common_assets::toml::TomlAssetPlugin;
use bevy_pg_calendar::prelude::{Calendar, CalendarNewHourEvent, Cron};
use serde::{Deserialize, Serialize, Deserializer, Serializer};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::fmt;

use super::pg_tasks::JobTasks;

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
        .register_type::<Job>()
        .register_type::<JobID>()
        .register_type::<JobData>()
        .register_type::<JobTasks>()
        .register_type::<JobStatus>()
        .register_type::<JobSchedule>()
        .register_type::<JobDebug>()
        .register_type::<JobPaused>()

        .add_plugins(JsonAssetPlugin::<JobData>::new(&["job.json"]))
        .add_plugins(TomlAssetPlugin::<JobData>::new(&["job.toml"]))
        .add_plugins(JsonAssetPlugin::<JobTrigger>::new(&["trigger.json"]))
        .add_plugins(TomlAssetPlugin::<JobTrigger>::new(&["trigger.toml"]))
        .add_plugins(JsonAssetPlugin::<JobTriggers>::new(&["triggers.json"]))
        .add_plugins(TomlAssetPlugin::<JobTriggers>::new(&["triggers.toml"]))

        .insert_resource(JobSettings::init(self.active, self.debug))
        .insert_resource(JobCatalog::init())
        .insert_resource(JobScheduler::init())
        .insert_resource(Jobs::init())

        .add_event::<StopJobEvent>()
        .add_event::<StartJobEvent>()

        .add_systems(Startup,   init)
        .add_systems(Update,    track.run_if(resource_exists::<LoadedJobDataHandles>
                                     .and(resource_exists::<LoadedJobTriggerHandles>)))

        .add_systems(PreUpdate, (trigger_jobs_calendar.run_if(on_event::<CalendarNewHourEvent>), 
                                 trigger_jobs_time)
                                .chain()
                                .run_if(if_jobs_active))

        .add_systems(PreUpdate, (stop_job.run_if(on_event::<StopJobEvent>), 
                                 start_job.run_if(on_event::<StartJobEvent>)).chain())
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
        first_task.add_task(commands, &entity);
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
#[derive(Resource, Reflect, Serialize, Deserialize, Clone)]
#[reflect(Resource)]
pub struct Jobs {
    data:   Vec<Job>
}
impl Jobs {
    fn init() -> Self {
        Self {data: Vec::new()}
    }
    pub fn add(&mut self, job: Job) {
        self.data.push(job); // This allows for multiple jobs per entity :o
    }

    pub fn get(&self, entity: &Entity) -> Option<&Job> {
        for job in self.data.iter(){
            if entity == &job.entity {
                return Some(job);
            }
        }
        return None;
    }

    pub fn get_mut(&mut self, entity: &Entity) -> Option<&mut Job> {
        for job in self.data.iter_mut(){
            if entity == &job.entity {
                return Some(job);
            }
        }
        return None;
    }

    pub fn fail_task(&mut self, commands: &mut Commands, task_entity: &Entity){
        if let Some(job) = self.get_mut(&task_entity) {
            let next_task_type = job.data.tasks.set_task(job.data.fail_task_id);
            next_task_type.add_task(commands, task_entity);
        } else {
            panic!("no entity {:?} in jobs", task_entity);
        }
    }

    pub fn next_task(&mut self, commands: &mut Commands, task_entity: &Entity) {
        if let Some(job) = self.get_mut(&task_entity) {
            let next_task_type = job.data.tasks.next_task();
            next_task_type.add_task(commands, task_entity);
        } else {
            panic!("no entity {:?} in jobs", task_entity);
            // info!("no entity {:?} in jobs", task_entity);
        }
    }
    pub fn jump_task(&mut self, commands: &mut Commands, task_entity: &Entity, next_task_id: u32){
        if let Some(job) = self.get_mut(&task_entity) {
            let next_task_type = job.data.tasks.set_task(next_task_id);
            next_task_type.add_task(commands, task_entity);
        } else {
            panic!("no entity {:?} in jobs", task_entity);
        }
    }
    pub fn index(&self, entity: &Entity) -> Option<usize> {
        return self.data.iter().position(|x| &x.entity == entity);
    }
    fn clean_task(&mut self, commands: &mut Commands, entity: &Entity){
        if let Some(job) = self.get(&entity){
            let task = job.data.tasks.get_current();
            task.remove(commands, entity);
        }
    }
    pub fn upsert(&mut self, commands: &mut Commands, entity: &Entity, job: Job) {
        if let Some(index) = self.index(entity){
            self.clean_task(commands, entity);
            self.data[index] = job;
        } else {
            self.data.push(job);
        }
    }
    pub fn remove(&mut self, commands: &mut Commands, job_id: JobID, entity: &Entity) {
        self.clean_task(commands, entity);
        self.data.retain(|x| !(&x.entity == entity && x.data.id == job_id))
    }
    pub fn remove_all(&mut self, entity: &Entity) {
        self.data.retain(|x| &x.entity != entity)
    }
    pub fn remove_all_clean(&mut self, commands: &mut Commands, entity: &Entity) {
        self.clean_task(commands, entity);
        self.data.retain(|x| &x.entity != entity)
    }
    pub fn clear(&mut self) {
        self.data.clear();
    }
    pub fn get_data(&self) -> &Vec<Job> {
        &self.data
    }
    pub fn pause(&mut self, commands: &mut Commands, entity: &Entity) {
        if let Some(job) = self.get_mut(entity){
            job.status = JobStatus::Paused;
            commands.entity(*entity).insert(JobPaused);
        }
    }
    pub fn unpause(&mut self, commands: &mut Commands, entity: &Entity) {
        if let Some(job) = self.get_mut(entity){
            job.status = JobStatus::Active;
            commands.entity(*entity).remove::<JobPaused>();
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct JobPaused;


#[derive(PartialEq, Copy, Clone, Serialize, Deserialize, Debug, Reflect)]
pub enum JobStatus {
    ToDo,
    Active,
    Done,
    Paused,
    Inactive
}

#[derive(Serialize, Deserialize, Asset, TypePath, Clone, Debug)]
pub struct JobTriggers {
    pub data: Vec<JobTrigger>
}

#[derive(Serialize, Deserialize, Asset, TypePath, Clone, Debug)]
pub struct JobTrigger {
    pub trigger_id:    u32,
    pub job_id:        JobID,
    pub schedule:      JobSchedule,
    pub active:        bool
}

#[derive(Serialize, Deserialize, Asset, Clone, Debug, Reflect)]
pub struct JobData {
    pub id:            JobID,
    pub label:         String,
    pub fail_task_id:  u32,               // ID of task to perform if task failed
    pub tasks:         JobTasks, 
}
impl JobData {
    pub fn assign(&self, commands: &mut Commands, entity: Entity, jobs: &mut ResMut<Jobs>) {
        jobs.remove_all_clean(commands, &entity);
        let mut job = Job::new(entity, self.clone());
        job.set_active();
        jobs.add(job);
        let first_task = self.tasks.get_current();
        first_task.add_task(commands, &entity);
    }


    pub fn start(&self, commands: &mut Commands, jobs: &mut ResMut<Jobs>) -> Entity{ 
        let first_task = self.tasks.get_current();
        let job_entity = first_task.spawn_with_task(commands);
        let mut job = Job::new(job_entity, self.clone());
        job.set_active();
        jobs.add(job);
        return job_entity;
    }
}

#[derive(Clone, Debug, Reflect, Serialize, Deserialize)]
pub struct Job {
    pub entity:        Entity,           
    loopk:             u32,              // Used for loops to count iterations
    status:            JobStatus,
    #[serde(serialize_with = "serialize_job_data")]
    pub data:          JobData,          // List of tasks to be performed by entity
}

impl Job {
    pub fn new(entity: Entity, data: JobData) -> Self {
        Job {
            entity,
            data,
            loopk: 0,
            status: JobStatus::ToDo
        }
    }
    pub fn loop_reset(&mut self){
        self.loopk = 0;
    }
    pub fn loop_incr(&mut self){
        self.loopk += 1;
    }
    pub fn loopk(&self) -> u32 {
        self.loopk
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


#[derive(Serialize, Asset, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct JobID(pub u32);

impl fmt::Display for JobID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "JobID: {}", self.0)
    }
}

impl JobID {
    pub fn from_str(job_string: &str) -> Self {
        let mut s = DefaultHasher::new();
        job_string.hash(&mut s);
        let hashed_id = JobID(s.finish() as u32);
        return hashed_id;
    }
}

impl<'de> Deserialize<'de> for JobID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_job_id: String = Deserialize::deserialize(deserializer)?;
        let mut s = DefaultHasher::new();
        string_job_id.hash(&mut s);
        let hashed_id = JobID(s.finish() as u32);
        info!("Job String: {} Hashed ID: {}", string_job_id, hashed_id);
        return Ok(hashed_id);
    }
}

#[derive(Serialize)]
struct SerializeJobData {
    id:           String,
    label:        String,
    fail_task_id: u32,
    tasks:        JobTasks
}

fn serialize_job_data<S>(jd: &JobData, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer
{
    let sjd = SerializeJobData {
        id: jd.label.clone(),
        label: jd.label.clone(),
        fail_task_id: jd.fail_task_id,
        tasks: jd.tasks.clone()
    };

    sjd.serialize(serializer)
}
