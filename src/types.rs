use bevy::prelude::*;
use dyn_clone::DynClone;
use serde::{de::Error, de::Unexpected};
use std::any::Any;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use bevy::platform::collections::HashMap;
use bevy::reflect::utility::GenericTypeInfoCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::fmt;
use serde::{Deserialize, Serialize, Deserializer, Serializer};

use bevy::reflect::{ApplyError, GetTypeRegistration, ReflectMut, ReflectOwned, ReflectRef, OpaqueInfo, TypeInfo, TypePath, Typed};

use crate::jobs::JobPaused;

#[typetag::serde(tag = "type")]
pub trait PGTask: Reflect + Any + Send + Sync + DynClone + Debug {
    fn insert(&self, commands: &mut Commands, entity: &Entity);
    fn remove(&self, commands: &mut Commands, entity: &Entity);
    fn spawn(&self, commands: &mut Commands) -> Entity;
}

dyn_clone::clone_trait_object!(PGTask);

#[derive(Debug, Reflect, Resource, Clone, Component, Serialize, Deserialize)]
pub struct Task {
    #[serde(skip_deserializing)]
    pub id:     u32,
    pub next:   Option<u32>,
    pub task:   Box<dyn PGTask + 'static>
}

#[derive(Debug, Reflect, Clone, Serialize, Deserialize)]
pub struct JobTasks {
    #[serde(deserialize_with="deserialize_jobtask_data")]    
    pub data: HashMap<u32, Task>,
    pub current_task_id: u32
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


#[derive(Serialize)]
struct SerializeJobData {
    id:           String,
    label:        String,
    fail_task_id: u32,
    tasks:        JobTasks
}


impl Default for JobTasks {
    fn default() -> Self {
        JobTasks{
            data: HashMap::default(), 
            current_task_id: 0
        }
    }
}

impl JobTasks { 
    pub fn new() -> Self {
        JobTasks{
            data: HashMap::default(),
            current_task_id: 0,
        }
    }

    pub fn first(
        &mut self, 
        task: Box<dyn PGTask>,
        next: Option<u32>
    ){
        let t: Task = Task{
            id: 0, 
            next,
            task
        };
        self.data.insert(0, t);
    }

    pub fn next(
        &mut self, 
        task: Box<dyn PGTask>, 
        next: Option<u32>
    ){
        let id = self.next_index();
        let t: Task = Task{
            id, 
            next,
            task
        };
        self.data.insert(id, t);
    }

    pub fn add_at(
        &mut self, 
        id:   u32,
        task: Box<dyn PGTask>, 
        next: Option<u32>
    ){
        let t: Task = Task{
            id, 
            next,
            task
        };
        self.data.insert(id, t);
    }

    fn next_index(&self) -> u32 {
        let Some(max_key) = self.data.keys().max() else {return 0};
        return *max_key+1;
    }

    pub fn add_task(&mut self, task: Task) {
        self.data.insert(task.id, task);
    }
    pub fn get_next_id(&mut self) -> u32 {
        let current_task_data = self.data.get(&self.current_task_id).unwrap();
        if let Some(next_task_id) = current_task_data.next {
            return next_task_id;
        } else {
            return current_task_data.id + 1;
        }
    }
    pub fn start(&mut self, commands: &mut Commands, job_entity: Entity) -> Entity {
        let current_task = &self.data.get(&self.current_task_id).unwrap();
        current_task.task.insert(commands, &job_entity);
        info!(" [Tasks]: Starting job for entity: {:?}", job_entity);
        return job_entity;
    }
    pub fn set_task(&mut self, next_task_id: u32) -> Option<&Task> {
        self.current_task_id = next_task_id;
        return self.get_current();
    }
    pub fn next_task(&mut self) -> Option<&Task> {
        self.current_task_id = self.get_next_id();
        return self.get_current();
    }
    pub fn get_current(&self) -> Option<&Task> {
        if let Some(task) = self.data.get(&self.current_task_id) {
            return Some(task);
        } else {
            error!("[JOBS] get current missing: {} ", self.current_task_id);
            return None;
        }
    }
}


fn deserialize_jobtask_data<'de, D>(deserializer: D) -> Result<HashMap<u32, Task>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_map = HashMap::<String, Task>::deserialize(deserializer)?;
    let original_len = str_map.len();
    let data = {
        str_map
            .into_iter()
            .map(|(str_key, mut value)| match str_key.parse() {
                Ok(int_key) => {
                    value.id = int_key;    
                    Ok((int_key, value))
                },
                Err(_) => Err({
                    Error::invalid_value(
                        Unexpected::Str(&str_key),
                        &"a non-negative integer",
                    )
                }),
            }).collect::<Result<HashMap<_, _>, _>>()?
    };
    // multiple strings could parse to the same int, e.g "0" and "00"
    if data.len() < original_len {
        return Err(Error::custom("detected duplicate integer key"));
    }
    Ok(data)
}

/// JobID creating ID from string in Job Data. Uses label to create ID
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
        info!(" [JOBS] Job String: {} Hashed ID: {}", string_job_id, hashed_id);
        return Ok(hashed_id);
    }
}

/// JobData is read from job.toml files
#[derive(Asset, Debug, Reflect, Clone, Serialize, Deserialize)]
pub struct JobData {
    pub id:            JobID,
    pub label:         String,
    pub fail_task_id:  u32,
    pub tasks:         JobTasks
}

impl JobData {
    pub fn assign(
        &self, 
        commands:  &mut Commands, 
        entity:    Entity
    ) {
        #[cfg(feature="verbose")]
        info!(" [JOBS] Assign JobData {} to {}", self.label, entity);

        let mut job = Job::new(self.clone());
        job.set_active();
        commands.entity(entity).insert(job);

        if let Some(first_task) = self.tasks.get_current(){
            first_task.task.insert(commands, &entity);
        } else {
            error!("Could not assign task to {}", entity);
        }
    }

    pub fn start(
        &self, 
        commands: &mut Commands
    ) -> Option<Entity>{ 
        info!(" [JOBS] Starting JobData {}", self.label);
        if let Some(first_task) = self.tasks.get_current(){
            let job_entity = first_task.task.spawn(commands);
            let mut job = Job::new(self.clone());
            job.set_active();
            commands.entity(job_entity).insert(job);
            return Some(job_entity);
        } else {
            return None;
        }
    }
}



#[derive(PartialEq, Copy, Clone, Debug, Reflect, Serialize, Deserialize)]
pub enum JobStatus {
    ToDo,
    Active,
    Done,
    Paused,
    Inactive
}

#[derive(Component, Debug, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Job {
    loopk:             u32,              // Used for loops to count iterations
    status:            JobStatus,
    #[serde(serialize_with = "serialize_job_data")]
    // #[serde(deserialize_with="deserialize_job_data")]
    pub data:          JobData,          // List of tasks to be performed by entity
}


impl Job {
    pub fn new(
        data:       JobData
    ) -> Self {
        Job {
            data,
            loopk: 0,
            status: JobStatus::ToDo,
        }
    }

    pub fn assign(
        &mut self, 
        commands:  &mut Commands, 
        entity:    Entity
    ) {
        #[cfg(feature="verbose")]
        info!(" [JOBS] Assign job {} to {}", self.data.label, entity);

        self.set_active();
        commands.entity(entity).insert(self.clone());
        if let Some(first_task) = self.data.tasks.get_current(){
            first_task.task.insert(commands, &entity);
        } else {
            error!("Could not assign first task to entity: {}", entity);
        }
    }

    pub fn start(
        &mut self, 
        commands: &mut Commands
    ) -> Option<Entity>{ 
        info!(" [JOBS] Starting job {}", self.data.label);
        self.set_active();
        if let Some(first_task) = self.data.tasks.get_current(){
            let job_entity = first_task.task.spawn(commands);
            commands.entity(job_entity).insert(self.clone());
            return Some(job_entity);
        } else {
            error!("Could not start job {}", self.data.label);
            return None;
        }

    }


    pub fn current_task(
        &self
    ) -> Option<&Task> {
        return self.data.tasks.get_current();
    }

    pub fn remove_current(
        &self,
        commands:    &mut Commands, 
        task_entity: &Entity
    ){
        if let Some(task) = self.current_task(){
            task.task.remove(commands, task_entity);
        }
    }

    pub fn fail_task(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity
    ) {
        self.remove_current(commands, task_entity);
        if let Some(next_task) = self.data.tasks.set_task(self.data.fail_task_id){
            next_task.task.insert(commands, task_entity);
        }
    }

    pub fn cancel(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity
    ) {
        self.remove_current(commands, task_entity);
        commands.entity(*task_entity).remove::<Job>();
    }

    pub fn next_task(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity
    ) {
        self.remove_current(commands, task_entity);
        if let Some(next_task) = self.data.tasks.next_task(){
            next_task.task.insert(commands, task_entity);
        }
    }

    pub fn jump_task(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity,
        next_task_id: u32
    ) {
        self.remove_current(commands, task_entity);
        if let Some(next_task) = self.data.tasks.set_task(next_task_id){
            next_task.task.insert(commands, task_entity);
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

    pub fn pause(
        &mut self, 
        commands:  &mut Commands, 
        entity:    &Entity
    ) {
        self.status = JobStatus::Paused;
        commands.entity(*entity).insert(JobPaused);
        
    }

    pub fn unpause(
        &mut self, 
        commands:  &mut Commands, 
        entity:    &Entity,
    ) {
        self.status = JobStatus::Active;
        commands.entity(*entity).remove::<JobPaused>();
        
    }

    pub fn label(&self) -> &str {
        &self.data.label
    }

}

impl GetTypeRegistration for Box<dyn PGTask> {
    fn get_type_registration() -> bevy::reflect::TypeRegistration {
        bevy::reflect::TypeRegistration::of::<Box<dyn PGTask>>()
    }
}

impl TypePath for Box<dyn PGTask> {
    fn type_path() -> &'static str {
        "std::boxed::Box(dyn bevy_pg_jobs::PGTask)"
    }

    fn short_type_path() -> &'static str {
        "Box(dyn PGTask)"
    }

    fn type_ident() -> Option<&'static str> {
        Some("Box")
    }

    fn crate_name() -> Option<&'static str> {
        Some("std")
    }

    fn module_path() -> Option<&'static str> {
        Some("std::boxed")
    }
}

impl Typed for Box<dyn PGTask> {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

impl PartialReflect for Box<dyn PGTask> {
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.as_ref().get_represented_type_info()
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self.as_ref()
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self.as_mut()
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        if let Some(a) = self.as_reflect().reflect_clone().ok(){
            Ok(a)
        } else {
            let c = self.into_partial_reflect();
            Err(c)
        }
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self.as_ref())
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self.as_mut())
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let Some(_task) = value.try_downcast_ref::<Self>() {
            // *self = task.clone(); // Ensure PGTask is Clone
            Ok(())
        } else {
            // Err(ApplyError::MismatchedTypes{from_type: Box::new(""), to_type: Box::new("b".to_string())})
            Err(ApplyError::DifferentSize { from_size: 0, to_size: 0 })
        }
    }

    fn reflect_ref(&self) -> ReflectRef {
        return self.as_ref().reflect_ref();
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        self.as_mut().reflect_mut()
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }
}

impl Reflect for Box<dyn PGTask> {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Box::new((*self).into_any())
    }

    fn as_any(&self) -> &dyn Any {
        self.deref().as_any()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.deref_mut().as_any_mut()
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        self.deref_mut().set(value)
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        return self.as_reflect().reflect_clone().ok().unwrap();
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self.deref().as_reflect()
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self.deref_mut().as_reflect_mut()
    }
}

impl FromReflect for Box<dyn PGTask> {

    fn from_reflect(
        _: &(dyn bevy::prelude::PartialReflect + 'static)
    ) -> std::option::Option<Self> { 
        return None;
    }

    fn take_from_reflect(
            reflect: Box<dyn PartialReflect>,
        ) -> std::result::Result<Self, Box<dyn PartialReflect>> {
        info!("{:?}", reflect);
        todo!();
    }

}
