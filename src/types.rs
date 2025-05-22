use bevy::prelude::*;
use dyn_clone::DynClone;
use bevy::ecs::entity::{MapEntities, EntityMapper};
use serde::{de::Error, de::Unexpected};
use bevy::ecs::reflect::ReflectMapEntities;
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
    fn insert_task(&self, commands: &mut Commands, entity: &Entity);
    fn remove(&self, commands: &mut Commands, entity: &Entity);
    fn spawn_with_task(&self, commands: &mut Commands) -> Entity;
}

dyn_clone::clone_trait_object!(PGTask);

#[derive(Debug, Reflect, Resource, Clone, Component, Serialize, Deserialize)]
pub struct Task {
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
        current_task.task.insert_task(commands, &job_entity);
        info!(" [Tasks]: Starting job for entity: {:?}", job_entity);
        return job_entity;
    }
    pub fn set_task(&mut self, next_task_id: u32) -> &Task {
        self.current_task_id = next_task_id;
        return self.get_current();
    }
    pub fn next_task(&mut self) -> &Task {
        self.current_task_id = self.get_next_id();
        return self.get_current();
    }
    pub fn get_current(&self) -> &Task {
        if let Some(task) = self.data.get(&self.current_task_id) {
            return task;
        } else {
            panic!("no task for {}", self.current_task_id);
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
        // info!("Job String: {} Hashed ID: {}", string_job_id, hashed_id);
        return Ok(hashed_id);
    }
}


#[derive(Asset, Debug, Reflect, Clone, Serialize, Deserialize)]
pub struct JobData {
    pub id:            JobID,
    pub label:         String,
    pub fail_task_id:  u32,
    // #[serde(serialize_with = "serialize_job_tasks")]
    pub tasks:         JobTasks
}

impl JobData {
    pub fn assign(
        &self, 
        commands:  &mut Commands, 
        entity:    Entity,
        jobs:      &mut ResMut<Jobs>
    ) {
        let new_index = jobs.get_new_index();
        let mut job = Job::new(new_index, self.clone());
        commands.entity(entity).insert(JobIndex(new_index));

        job.set_active();
        jobs.add(job);
        let first_task = self.tasks.get_current();
        first_task.task.insert_task(commands, &entity);
    }

    pub fn start(
        &self, 
        commands: &mut Commands, 
        jobs: &mut ResMut<Jobs>
    ) -> Entity{ 
        let first_task = self.tasks.get_current();
        let job_entity = first_task.task.spawn_with_task(commands);
        let new_index = jobs.get_new_index();
        let mut job = Job::new(new_index, self.clone());
        commands.entity(job_entity).insert(JobIndex(new_index));

        job.set_active();
        jobs.add(job);
        return job_entity;
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

#[derive(Resource, Debug, Reflect, Clone, Serialize, Deserialize, MapEntities)]
#[reflect(MapEntities)]
pub struct Job {
    #[entities]
    pub entity:        Option<Entity>,
    pub index:         u32,      
    loopk:             u32,              // Used for loops to count iterations
    status:            JobStatus,
    #[serde(serialize_with = "serialize_job_data")]
    // #[serde(deserialize_with="deserialize_job_data")]
    pub data:          JobData,          // List of tasks to be performed by entity
}

// impl MapEntities for Job {
//     fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
//         // info!("Mapping job entity");
//         if self.entity.is_some(){
//             self.entity = Some(entity_mapper.get_mapped(self.entity.unwrap()));
//         }
//     }
// }

#[derive(Component, Reflect, Clone, Copy)]
#[reflect(Component)]
pub struct JobIndex(pub u32);


impl Job {
    pub fn new(
        index:      u32,
        data:       JobData
    ) -> Self {
        Job {
            entity: None,
            index,
            data,
            loopk: 0,
            status: JobStatus::ToDo,
        }
    }
    pub fn set_entity(&mut self, entity: Entity) {
        self.entity = Some(entity);
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

#[derive(Resource, Reflect, Serialize, Deserialize, Clone)]
#[reflect(Resource)]
pub struct Jobs {
    pub        data:   Vec<Job>,
    pub(crate) current_index: u32
}

impl Jobs {
    pub(crate) fn init() -> Self {
        Self {
            data: Vec::new(),
            current_index: 0
        }
    }

    pub fn get_new_index(&mut self) -> u32 {
        self.current_index += 1;
        return self.current_index;
    }

    pub fn add(&mut self, job: Job) {
        self.data.push(job); // This allows for multiple jobs per entity :o
    }

    pub fn get(&self, index: u32) -> Option<&Job> {
        for job in self.data.iter() {
            if index == job.index {
                return Some(job);
            }
        }
        return None;
    }

    pub fn get_mut(&mut self, index: &JobIndex) -> Option<&mut Job> {
        for job in self.data.iter_mut() {
            if index.0 == job.index {
                return Some(job);
            }
        }
        return None;
    }

    pub fn possible_next_task(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity,
        job_index:   &JobIndex
    ) {
        if let Some(job) = self.get_mut(job_index) {
            let next_task_type = job.data.tasks.next_task();
            next_task_type.task.insert_task(commands, task_entity);
        }
    }

    pub fn fail_task(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity,
        job_index:   &JobIndex
    ) {
        if let Some(job) = self.get_mut(job_index) {
            let next_task_type = job.data.tasks.set_task(job.data.fail_task_id);
            next_task_type.task.insert_task(commands, task_entity);
        } else {
            panic!("no entity in jobs {:?} ", task_entity);
        }
    }

    pub fn next_task(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity,
        job_index:   &JobIndex
    ) {
        if let Some(job) = self.get_mut(job_index) {
            let next_task_type = job.data.tasks.next_task();
            // info!("next task for Entity: {:?}", task_entity);
            next_task_type.task.insert_task(commands, task_entity);
        } else {
            panic!("next task: no entity in jobs {:?} Entity: {:?}", task_entity, task_entity);
        }
    }

    pub fn jump_task(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity,
        job_index:   &JobIndex,
        next_task_id: u32
    ) {
        if let Some(job) = self.get_mut(job_index) {
            let next_task_type = job.data.tasks.set_task(next_task_id);
            next_task_type.task.insert_task(commands, task_entity);
        } else {
            panic!("no entity {:?} in jobs", task_entity);
        }
    }

    pub fn index(&self, job_index: &JobIndex) -> Option<usize> {
        return self.data.iter().position(|x| x.index == job_index.0);
    }

    fn clean_task(
        &mut self, 
        commands:  &mut Commands, 
        entity:    &Entity,
        job_index: &JobIndex
    ) {
        if let Some(job) = self.get(job_index.0) {
            let task = job.data.tasks.get_current();
            // info!("cleaning task {:?} from job: {} for entity: {:?}", task, job.data.Entity, entity);
            task.task.remove(commands, entity);
        }
    }

    pub fn upsert(
        &mut self, 
        commands:  &mut Commands, 
        entity:    &Entity, 
        job_index: &JobIndex,
        job:       Job
    ) {
        if let Some(index) = self.index(job_index) {
            self.clean_task(commands, entity, job_index);
            self.data[index] = job;
        } else {
            self.data.push(job);
        }
    }
    pub fn remove(
        &mut self, 
        commands:  &mut Commands, 
        job_id:    JobID, 
        entity:    &Entity,
        job_index: &JobIndex
    ) {
        self.clean_task(commands, entity, job_index);
        self.data
            .retain(|x| !(x.index == job_index.0 && x.data.id == job_id))
    }

    pub fn remove_all(&mut self, job_index: &JobIndex) {
        // info!("in remove all? {:?}", entity);
        self.data.retain(|x| x.index != job_index.0)
    }

    pub fn remove_all_clean(
        &mut self, 
        commands:  &mut Commands, 
        entity:    &Entity, 
        job_index: &JobIndex,
    ) {
        self.clean_task(commands, entity, job_index);
        self.data.retain(|x| x.index != job_index.0);
        commands.entity(*entity).remove::<JobIndex>();
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn get_data(&self) -> &Vec<Job> {
        &self.data
    }

    pub fn pause(
        &mut self, 
        commands:  &mut Commands, 
        entity:    &Entity, 
        job_index: &JobIndex
    ) {
        if let Some(job) = self.get_mut(job_index) {
            // info!("pausing job for Entity {:?}", entity);
            job.status = JobStatus::Paused;
            commands.entity(*entity).insert(JobPaused);
        }
    }

    pub fn unpause(
        &mut self, 
        commands:  &mut Commands, 
        entity:    &Entity,
        job_index: &JobIndex
    ) {
        if let Some(job) = self.get_mut(job_index) {
            // info!("unpausing job for Entity {:?}", entity);
            job.status = JobStatus::Active;
            commands.entity(*entity).remove::<JobPaused>();
        }
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
    fn from_reflect(_: &(dyn bevy::prelude::PartialReflect + 'static)) -> std::option::Option<Self> { 
        return None;
    }

    fn take_from_reflect(
            reflect: Box<dyn PartialReflect>,
        ) -> std::result::Result<Self, Box<dyn PartialReflect>> {
        todo!();
    }

}
