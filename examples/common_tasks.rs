
use bevy::ecs::component::{Component, ComponentStorage, TableStorage};
use bevy::ecs::storage::Table;
use bevy::ecs::system::Resource;
use bevy::math::{Vec3, Vec2};
use bevy::utils::HashMap;

use bevy_pg_jobs::prelude::{Job, JobSchedule};


fn main(){}

// use self::::jobs::{Job, JobSchedule};
// use crate::tasks::PGTask;
/* 
#[derive(PGTask)]
pub struct SpawnTask {
    // msd:      MSD Dont have msd here
    angle:  f32, // Later can be just Transform but,
    loc:    Vec3
}

#[derive(PGTask)]
pub struct WaitTask {
    pub schedule: JobSchedule
}

#[derive(PGTask)]
pub struct MoveTask {
    // pub source:         Vec2,
    // pub target:         Vec2,
    // pub route:          Option<Route>,
    // pub route_index:    u8
}

#[derive(PGTask)]
pub struct RotateTask;

#[derive(PGTask)]
pub struct DespawnTask;

#[derive(PGTask)]
pub struct HideTask;

#[derive(PGTask)]
pub struct ShowTask;

#[derive(PGTask)]
pub struct TeleportTask;

#[derive(PGTask)]
pub struct SendEventTask;

#[derive(PGTask)]
pub struct InsertComponentTask;

#[derive(PGTask)]
pub struct DecisionTask;

#[derive(PGTask)]
pub struct LoopNKTask; // Loop N steps K times

#[derive(PGTask)]
pub struct LoopNTask; // Loop N steps (until something inside breaks the loop)

*/