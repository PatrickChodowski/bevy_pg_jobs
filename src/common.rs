
// Collection of very common task implementations
use bevy::prelude::*;
use bevy_pg_calendar::prelude::Calendar;
use rand::Rng;
use serde::{Serialize, Deserialize};

use crate::prelude::{PGTask, Job, JobSchedule};
use pg_jobs_macros::PGTask;


#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component, Serialize, Deserialize)]
pub struct DespawnTask;

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[reflect(Serialize)]
pub struct HideTask;

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[reflect(Serialize)]
pub struct ShowTask;

#[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[reflect(Serialize)]
pub struct WaitTask {
    pub schedule: JobSchedule
}

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub struct RandomWaitTask{
    min: f32,
    max: f32
}
impl RandomWaitTask {
    pub fn new(min: f32, max: f32) -> Self {
        Self {min, max}
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub struct TeleportTask {
    pub loc: Vec3
}

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub struct LoopTask {
    pub start_id:  u32, // Loops the tasks specified in the vector
    pub maxk:      Option<u32>
}
impl Default for LoopTask {
    fn default() -> Self {
        LoopTask{start_id: 0, maxk: None}
    }
}

pub fn despawn_task(
    mut commands:       Commands,
    tasks:              Query<Entity, With<DespawnTask>>
){
    for entity in tasks.iter(){
        commands.entity(entity).despawn();
    }
}

pub fn loop_task(
    mut commands:   Commands,
    mut tasks:      Query<(Entity, &LoopTask, &mut Job)>
){
    for (task_entity, loop_task, mut job) in tasks.iter_mut(){
        if let Some(maxk) = loop_task.maxk {
            if job.loopk() >= maxk {
                job.loop_reset();
                job.next_task(&mut commands, &task_entity); 
            } else {
                job.loop_incr();
                job.jump_task(&mut commands, &task_entity, loop_task.start_id); 
            }
        }
    }
}

pub fn show_task(
    mut commands:   Commands,
    mut tasks:      Query<(Entity, &mut Visibility, &mut Job), With<ShowTask>>
){
    for (task_entity, mut vis, mut job) in tasks.iter_mut(){
        *vis = Visibility::Inherited;
        job.next_task(&mut commands, &task_entity);
    }
}

pub fn hide_task(
    mut commands:   Commands,
    mut tasks:      Query<(Entity, &mut Visibility, &mut Job), With<HideTask>>
){
    for (task_entity, mut vis, mut job) in tasks.iter_mut(){
        *vis = Visibility::Hidden;
        job.next_task(&mut commands, &task_entity);
    }
}

pub fn teleport_task(
    mut commands:   Commands,
    mut tasks:      Query<(Entity, &mut Transform, &TeleportTask, &mut Job)>
){
    for (task_entity, mut transform, teleport_task, mut job) in tasks.iter_mut(){
        transform.translation = teleport_task.loc;
        job.next_task(&mut commands, &task_entity);
    }  
}

pub fn random_wait_task(
    mut commands:      Commands,
    mut tasks:         Query<(Entity, &RandomWaitTask)>,
){
    for (task_entity, random_wait_task) in tasks.iter_mut(){
        let mut rng = rand::rng();
        let random_delay: f32 = rng.random_range(random_wait_task.min..=random_wait_task.max);
        commands.entity(task_entity).insert(WaitTask{schedule: JobSchedule::RealDelay(random_delay)});
        commands.entity(task_entity).remove::<RandomWaitTask>();
    }
}

pub fn wait_task_time(
    mut commands:   Commands,
    time:           Res<Time>,
    mut tasks:      Query<(Entity, &mut WaitTask, &mut Job)>,
){
    for (task_entity, mut wait_task, mut job) in tasks.iter_mut(){
        match &mut wait_task.schedule {
            JobSchedule::RealDelay(delay) => {
                if *delay > 0.0 {
                    *delay -= time.delta_secs();
                } else {
                    job.next_task(&mut commands, &task_entity);
                    commands.entity(task_entity).remove::<WaitTask>();
                }
            }
            _ => {}
        }
    }
}


pub fn wait_idle_calendar(
    mut commands: Commands,
    calendar:     Res<Calendar>,
    mut tasks:    Query<(Entity, &mut WaitTask, &mut Job)>
){
    for (task_entity, mut wait_task, mut job) in tasks.iter_mut(){
        match &mut wait_task.schedule {
                JobSchedule::Cron(cron) => {
                    if cron.is_time(&calendar){
                        job.next_task(&mut commands, &task_entity);
                        commands.entity(task_entity).remove::<WaitTask>();
                    }
                 }
                 JobSchedule::Delay(delay) => {
                    if *delay > 0 {
                        *delay -= 1;
                    } else {
                        commands.entity(task_entity).remove::<WaitTask>();
                        job.next_task(&mut commands, &task_entity);
                    }
                }
                _=> {}   
        }
    }
}
