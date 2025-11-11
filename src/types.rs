use bevy::prelude::*;
use dyn_clone::DynClone;
use std::any::Any;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use bevy::platform::collections::HashMap;
use bevy::reflect::utility::GenericTypeInfoCell;

use bevy::reflect::{ApplyError, GetTypeRegistration, ReflectMut, ReflectOwned, ReflectRef, OpaqueInfo, TypeInfo, TypePath, Typed};

use crate::jobs::JobPaused;

// #[typetag::serde(tag = "type")]
pub trait PGTask: Reflect + Any + Send + Sync + DynClone + Debug {
    fn insert(&self, commands: &mut Commands, entity: &Entity);
    fn remove(&self, commands: &mut Commands, entity: &Entity);
    fn spawn(&self, commands: &mut Commands) -> Entity;
}

dyn_clone::clone_trait_object!(PGTask);

#[derive(Debug, Reflect, Resource, Clone, Component)]
pub struct Task {
    pub id:     u32,
    pub next:   Option<u32>,
    pub task:   Box<dyn PGTask + 'static>
}

#[derive(Debug, Reflect, Clone)]
pub struct JobTasks {
    pub data: HashMap<u32, Task>,
    pub current_task_id: u32,
    last_added: u32
}

impl Default for JobTasks {
    fn default() -> Self {
        JobTasks{
            data: HashMap::default(), 
            current_task_id: 0,
            last_added: 0
        }
    }
}

impl JobTasks { 
    pub fn from_vec(v: Vec<Box<dyn PGTask>>) -> Self {
        let mut jt = JobTasks::new();
        for (index, task) in v.iter().enumerate(){
            if index == 0 {
                jt.first(task.clone());
            } else {
                jt.next(task.clone());
            }

        }
        return jt;
    }


    pub fn new() -> Self {
        JobTasks{
            data: HashMap::default(),
            last_added: 0,
            current_task_id: 0,
        }
    }

    pub fn with_next(&mut self, next: u32){
        self.data.get_mut(&self.last_added).unwrap().next = Some(next);
    }

    pub fn first(
        &mut self, 
        task: Box<dyn PGTask>
    ) -> &mut Self {
        let t: Task = Task{
            id: 0, 
            next: None,
            task
        };
        self.data.insert(0, t);
        self.last_added = 0;
        return self;
    }

    pub fn next(
        &mut self, 
        task: Box<dyn PGTask>
    ) -> &mut Self {
        let id = self.next_index();
        let t: Task = Task{
            id, 
            next: None,
            task
        };
        self.data.insert(id, t);
        self.last_added = id;
        return self;
    }

    pub fn add_at(
        &mut self, 
        id:   u32,
        task: Box<dyn PGTask>
    ) -> &mut Self {
        let t: Task = Task{
            id, 
            next: None,
            task
        };
        self.data.insert(id, t);
        self.last_added = id;
        return self;
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
        #[cfg(feature="verbose")]
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
            #[cfg(feature="verbose")]
            warn!("[JOBS] get current missing: {} ", self.current_task_id);
            return None;
        }
    }
}

#[derive(Clone, Copy, Default, Reflect, Debug)]
pub enum JobOnFail {
    #[default]
    Cancel,
    RunTask(u32),
    Nothing,
    Despawn
}

/// JobData is read from job.toml files
#[derive(Asset, Debug, Reflect, Clone)]
pub struct JobData {
    /// Ideally unique name
    pub name:          &'static str,
    pub on_fail:       JobOnFail,
    pub tasks:         JobTasks
}

impl JobData {
    pub fn assign(
        &self, 
        commands:  &mut Commands, 
        entity:    Entity
    ) {
        #[cfg(feature="verbose")]
        info!(" [JOBS] Assign JobData {} to {}", self.name, entity);

        let mut job = Job::new(self.clone());
        job.set_active();
        commands.entity(entity).insert(job);

        if let Some(first_task) = self.tasks.get_current(){
            first_task.task.insert(commands, &entity);
        } else {
            #[cfg(feature="verbose")]
            warn!("Could not assign task to {}", entity);
        }
    }

    pub fn start(
        &self, 
        commands: &mut Commands
    ) -> Option<Entity>{ 
        #[cfg(feature="verbose")]
        info!(" [JOBS] Starting JobData {}", self.name);
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



#[derive(PartialEq, Copy, Clone, Debug, Reflect)]
pub enum JobStatus {
    ToDo,
    Active,
    Done,
    Paused,
    Inactive
}

#[derive(Component, Debug, Reflect, Clone)]
#[reflect(Component)]
pub struct Job {
    loopk:             u32,              // Used for loops to count iterations
    status:            JobStatus,
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
        info!(" [JOBS] Assign job {} to {}", self.data.name, entity);

        self.set_active();
        commands.entity(entity).insert(self.clone());
        if let Some(first_task) = self.data.tasks.get_current(){
            first_task.task.insert(commands, &entity);
        } else {
            #[cfg(feature="verbose")]
            warn!("Could not assign first task to entity: {}", entity);
        }
    }

    pub fn start(
        &mut self, 
        commands: &mut Commands
    ) -> Option<Entity>{ 
        #[cfg(feature="verbose")]
        info!(" [JOBS] Starting job {}", self.data.name);
        self.set_active();
        if let Some(first_task) = self.data.tasks.get_current(){
            let job_entity = first_task.task.spawn(commands);
            commands.entity(job_entity).insert(self.clone());
            return Some(job_entity);
        } else {
            error!(" [JOBS] Could not start job {}", self.data.name);
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

    pub fn fail(
        &mut self, 
        commands:    &mut Commands, 
        task_entity: &Entity
    ) {
        match self.data.on_fail {
            JobOnFail::Nothing => {}
            JobOnFail::Despawn => {
                commands.entity(*task_entity).despawn();
            }
            JobOnFail::Cancel => {
                self.cancel(commands, task_entity);
            }
            JobOnFail::RunTask(task_id) => {
                if let Some(next_task) = self.data.tasks.set_task(task_id){
                    next_task.task.insert(commands, task_entity);
                } else {
                    self.cancel(commands, task_entity);
                }
            }
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
        } else {
            commands.entity(*task_entity).remove::<Job>();
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

    pub fn name(&self) -> &str {
        &self.data.name
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
