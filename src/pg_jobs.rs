use bevy::app::{App, Plugin, PreUpdate, Update, Startup};
use bevy::asset::{Asset, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState, Handle};
use bevy::ecs::schedule::common_conditions::{on_event, resource_changed, resource_exists};
use bevy::ecs::schedule::{IntoSystemConfigs, Condition};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{Event, EventReader};
use bevy::ecs::component::Component;
use bevy::ecs::system::{Commands, Local, Resource, Res, ResMut, Query};
use bevy::ecs::query::With;
use bevy::sprite::Anchor;
use bevy::transform::components::Transform;
use bevy::hierarchy::DespawnRecursiveExt;
use bevy::utils::default;
use bevy::hierarchy::{BuildChildren, Parent};
use bevy::log::info;
use bevy::reflect::TypePath;
use bevy::render::color::Color;
use bevy::text::{Text2dBundle, TextStyle, Text};
use bevy_common_assets::json::JsonAssetPlugin;
use bevy_common_assets::toml::TomlAssetPlugin;
use bevy_pg_calendar::prelude::{Calendar, CalendarNewHourEvent, Cron};
use serde::{Deserialize, Serialize};

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
        .add_plugins(JsonAssetPlugin::<JobData>::new(&["jobdata.json"]))
        .add_plugins(TomlAssetPlugin::<JobData>::new(&["jobdata.toml"]))
        .add_plugins(JsonAssetPlugin::<JobTrigger>::new(&["trigger.json"]))
        .add_plugins(TomlAssetPlugin::<JobTrigger>::new(&["trigger.toml"]))

        .insert_resource(JobSettings::init(self.active, self.debug))
        .insert_resource(JobCatalog::init())
        .insert_resource(JobScheduler::init())
        .insert_resource(Jobs::init())

        .add_event::<StopJobEvent>()
        .add_event::<StartJobEvent>()

        .add_systems(Startup,   init)
        .add_systems(Update,    track.run_if(resource_exists::<LoadedJobDataHandles>
                                     .and_then(resource_exists::<LoadedJobTriggerHandles>)))

        .add_systems(PreUpdate, (trigger_jobs_calendar.run_if(on_event::<CalendarNewHourEvent>()), 
                                 trigger_jobs_time)
                                .chain()
                                .run_if(if_jobs_active))

        .add_systems(Update,    debug_jobs.run_if(if_jobs_debug.and_then(resource_changed::<Jobs>)))
        .add_systems(PreUpdate, (stop_job.run_if(on_event::<StopJobEvent>()), 
                                 start_job.run_if(on_event::<StartJobEvent>())).chain())
        ;
    }
}


#[derive(Event)]
pub struct StopJobEvent {
    pub entity: Entity
}

#[derive(Event)]
pub struct StartJobEvent {
    pub job_id: u32,
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
    mut jobs_catalog:       ResMut<JobCatalog>,
    mut jobs_scheduler:     ResMut<JobScheduler>,
    loaded_jobdata:         Res<LoadedJobDataHandles>,
    loaded_jobtrigger:      Res<LoadedJobTriggerHandles>
){

    if !job_ready.data_ready {
        if let Some(scenes_load_state) = ass.get_recursive_dependency_load_state(&loaded_jobdata.0) {
            if scenes_load_state == RecursiveDependencyLoadState::Loaded {
                job_ready.data_ready = true;
            }
        }
    }
    if !job_ready.triggers_ready {
        if let Some(scenes_load_state) = ass.get_recursive_dependency_load_state(&loaded_jobtrigger.0) {
            if scenes_load_state == RecursiveDependencyLoadState::Loaded {
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
    pub fn get(&self, id: u32) -> JobData {
        for job in self.data.iter() {
            if job.id == id {
                return job.clone();
            }
        }
        panic!("Missing job id in the catalog: {}", id);
    }
    pub fn assign(&self, job_id: u32, entity: Entity, jobs: &mut ResMut<Jobs>){
        let jobdata = self.get(job_id);
        let mut job = Job::new(entity, jobdata);
        job.set_active();
        jobs.add(job);
    }
    pub fn start(&self, commands: &mut Commands, job_id: u32, jobs: &mut ResMut<Jobs>) -> Entity {
        let jobdata = self.get(job_id);
        let first_task = jobdata.tasks.get_current();
        let job_entity = first_task.spawn_with_task(commands);
        let mut job = Job::new(job_entity, jobdata);
        job.set_active();
        jobs.add(job);
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
#[derive(Resource)]
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
    pub fn index(&self, entity: Entity) -> Option<usize> {
        return self.data.iter().position(|x| x.entity == entity);
    }
    pub fn upsert(&mut self, entity: Entity, job: Job) {
        if let Some(index) = self.index(entity){
            self.data[index] = job;
        } else {
            self.data.push(job);
        }
    }
    pub fn remove(&mut self, job_id: u32, entity: &Entity) {
        self.data.retain(|x| !(&x.entity == entity && x.data.id == job_id))
    }
    pub fn remove_all(&mut self, entity: &Entity) {
        self.data.retain(|x| &x.entity != entity)
    }
    pub fn clear(&mut self) {
        self.data.clear();
    }
    pub fn get_data(&self) -> &Vec<Job> {
        &self.data
    }
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize, Debug)]
pub enum JobStatus {
    ToDo,
    Active,
    Done,
    Paused,
    Inactive
}

#[derive(Serialize, Deserialize, Asset, TypePath, Clone, Debug)]
pub struct JobTrigger {
    pub trigger_id:    u32,
    pub job_id:        u32,
    pub schedule:      JobSchedule,
    pub active:        bool
}

#[derive(Serialize, Deserialize, Asset, TypePath, Clone, Debug)]
pub struct JobData {
    pub id:            u32,
    pub fail_task_id:  u32,               // ID of task to perform if task failed
    pub fail_job_id:   u32,               // ID of task to perform if job failed to start 
    pub tasks:         JobTasks, 
}

#[derive(Clone, Debug)]
pub struct Job {
    pub entity:        Entity,           
    loopk:             u32,              // Used for loops to count iterations
    status:            JobStatus,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    jobdebugs:          Query<(Entity, &Parent), With<JobDebug>>
){
    for ev in stop_job.read(){
        if let Some(job) = jobs.get(&ev.entity){
            let task = job.data.tasks.get_current();
            task.remove(&mut commands, ev.entity);
        }

        info!(" [JOBS] Removing all jobs for entity: {:?}", ev.entity);
        jobs.remove_all(&ev.entity);

        for (text_entity, task_entity) in jobdebugs.iter(){
            if **task_entity == ev.entity {
                commands.entity(text_entity).despawn_recursive();
                break;
            }
        }
    }

}

fn start_job(
    mut jobs:           ResMut<Jobs>,
    jobs_catalog:       Res<JobCatalog>,
    mut start_job:      EventReader<StartJobEvent>
){
    for ev in start_job.read(){
        info!(" [JOBS] Adding job {} to entity {:?}", ev.job_id, ev.entity);
        jobs_catalog.assign(ev.job_id, ev.entity, &mut jobs);
    }
}



#[derive(Component)]
struct JobDebug;



// Displays each entity current task and its parameters
fn debug_jobs(
    mut commands:     Commands, 
    ass:              Res<AssetServer>,
    jobs:             Res<Jobs>, 
    job_settings:     Res<JobSettings>,
    mut jobdebugs:    Query<(Entity, &Parent, &mut Text), With<JobDebug>>
){
    
    let font = ass.load("fonts/FiraMono-Medium.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 40.0,
        color: Color::BLACK,
    };

    if job_settings.get_debug(){

        for job in jobs.data.iter(){

            let current_task = job.data.tasks.get_current();
            let mut needs_label: bool = true;
    
            for(_text_entity, text_parent, mut text) in jobdebugs.iter_mut(){
    
                if **text_parent != job.entity {
                    continue; 
                }
                needs_label = false;
                text.sections[0].value = current_task.display();
            } 
    
            if needs_label {
                let debug_text = commands.spawn((
                    Text2dBundle {
                        text: Text::from_section(current_task.display(), text_style.clone()),
                        text_anchor: Anchor::TopCenter,
                        transform: Transform::from_xyz(0.0, 70.0, 10.0),
                        ..default()
                    },
                    JobDebug,
                )).id();
            
                commands.entity(job.entity).add_child(debug_text);
            }
        }
    } else {
        for(text_entity, _text_parent, _text) in jobdebugs.iter(){
            commands.entity(text_entity).despawn();
        }
    }
}
