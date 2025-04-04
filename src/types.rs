use bevy::prelude::*;
use std::any::Any;
use std::fmt::Debug;
use bevy::platform_support::collections::HashMap;

use bevy::reflect::{TypeInfo, TypePath, Typed, DynamicTyped, 
    ReflectMut, ReflectRef, ReflectOwned, ReflectCloneError, ApplyError};
use std::sync::OnceLock;

pub trait PGTask: Reflect + Any + Send + Sync + Debug {}

#[derive(Debug, Reflect, Resource, Component)]
// #[reflect(Component)]
pub struct Task {
    pub id:     u32,
    pub next:   Option<u32>,
    pub task:   Box<dyn PGTask + 'static>
}

#[derive(Debug, Reflect)]
pub struct JobTasks {
    pub data: HashMap<u32, Task>,
    pub current_task_id: u32
}

#[derive(Asset, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct JobID(pub u32);

#[derive(Debug, Reflect)]
pub struct JobData {
    pub id:            JobID,
    pub label:         String,
    pub fail_task_id:  u32,
    pub tasks:         JobTasks
}

#[derive(PartialEq, Copy, Clone, Debug, Reflect)]
pub enum JobStatus {
    ToDo,
    Active,
    Done,
    Paused,
    Inactive
}

#[derive(Debug, Reflect)]
pub struct Job {
    pub entity:        Entity,           
    loopk:             u32,              // Used for loops to count iterations
    status:            JobStatus,
    pub data:          JobData,          // List of tasks to be performed by entity
}


#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct Jobs {
    data:   Vec<Job>
}

// Really Need to implement Reflect
impl TypePath for Box<dyn PGTask> {
    fn type_path() -> &'static str {
        "std::boxed::Box(dyn bevy_reflect::Reflect)"
    }

    fn short_type_path() -> &'static str {
        "Box(dyn Reflect)"
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
    fn type_info() -> &'static TypeInfo { todo!() }
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
        if let Some(task) = value.try_downcast_ref::<Self>() {
            // *self = task.clone(); // Ensure PGTask is Clone
            Ok(())
        } else {
            // Err(ApplyError::MismatchedTypes{from_type: Box::new(""), to_type: Box::new("b".to_string())})
            Err(ApplyError::DifferentSize { from_size: 0, to_size: 0 })
        }
    }

    fn reflect_ref(&self) -> ReflectRef {
        self.as_ref().reflect_ref()
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        self.as_mut().reflect_mut()
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }
}



use std::ops::{Deref, DerefMut};



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









// impl Reflect for Box<dyn PGTask> {
//     fn into_any(self: Box<Self>) -> Box<dyn Any> {
//         self
//     }

//     fn as_any(&self) -> &dyn Any {
//         self.as_reflect().as_any()
//     }

//     fn as_any_mut(&mut self) -> &mut dyn Any {
//         self.as_reflect_mut().as_any_mut()
//     }

//     fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
//         self
//     }

//     fn as_reflect(&self) -> &dyn Reflect {
//         self.as_ref()
//     }

//     fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
//         self.as_mut()
//     }

//     fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
//         if let Ok(task) = value.downcast::<Self>() {
//             *self = task;
//             Ok(())
//         } else {
//             Err(value)
//         }
//     }
// }
