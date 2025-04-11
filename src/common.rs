
// Collection of very common task implementations
use bevy::prelude::*;
use bevy_pg_calendar::prelude::Calendar;
use rand::Rng;
use serde::{Serialize, Deserialize};

use crate::jobs::JobSchedule;
use crate::types::{Jobs, JobIndex};
use crate::prelude::PGTask;
use pg_jobs_macros::PGTask;


#[derive(Component, Resource, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component, Serialize, Deserialize)]
#[reflect(Resource)]
pub struct DespawnTask;


#[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub struct DespawnWithDelay{
    // pub timer: Timer
}
// impl DespawnWithDelay {
//     pub fn new(delay: f32) -> Self {
//         Self {timer: Timer::from_seconds(delay, TimerMode::Once)}
//     }
// }

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub struct HideTask;

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub struct ShowTask;

#[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize, PGTask)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
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

// pub fn despawn_with_delay_task(
//     mut commands:       Commands,
//     mut jobs:           ResMut<Jobs>,
//     time:               Res<Time>,
//     mut tasks:          Query<(Entity, &mut DespawnWithDelay)>
// ){
//     for (entity, mut dwd) in tasks.iter_mut(){
//         dwd.timer.tick(time.delta());
//         if dwd.timer.finished(){
//             commands.entity(entity).despawn();
//             jobs.remove_all(&entity);
//         }
//     }
// }

pub fn despawn_task(
    mut commands:       Commands,
    mut jobs:           ResMut<Jobs>,
    tasks:              Query<(Entity, &JobIndex), With<DespawnTask>>
){
    for (entity, job_index) in tasks.iter(){
        commands.entity(entity).despawn();
        jobs.remove_all(job_index);
    }
}

pub fn loop_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    tasks:          Query<(Entity, &LoopTask, &JobIndex)>
){
    for (task_entity, loop_task, job_index) in tasks.iter(){

        if let Some(maxk) = loop_task.maxk {
            if let Some(job) = jobs.get_mut(job_index.0){
                // final iteration
                if job.loopk() >= maxk {
                    job.loop_reset();
                    commands.entity(task_entity).remove::<LoopTask>();
                    jobs.next_task(&mut commands, &task_entity, job_index); 
                } else {
                    job.loop_incr();
                    commands.entity(task_entity).remove::<LoopTask>();
                    jobs.jump_task(&mut commands, &task_entity, job_index, loop_task.start_id); 
                }
            }
        }
    }
}

pub fn show_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    mut tasks:      Query<(Entity, &mut Visibility, &JobIndex), With<ShowTask>>
){
    for (task_entity, mut vis, job_index) in tasks.iter_mut(){
        *vis = Visibility::Inherited;
        commands.entity(task_entity).remove::<ShowTask>();
        jobs.next_task(&mut commands, &task_entity, job_index);
    }
}

pub fn hide_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    mut tasks:      Query<(Entity, &mut Visibility, &JobIndex), With<HideTask>>
){
    for (task_entity, mut vis, job_index) in tasks.iter_mut(){
        *vis = Visibility::Hidden;
        commands.entity(task_entity).remove::<HideTask>();
        jobs.next_task(&mut commands, &task_entity, job_index);
    }
}

pub fn teleport_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    mut tasks:      Query<(Entity, &mut Transform, &TeleportTask, &JobIndex)>
){
    for (task_entity, mut transform, teleport_task, job_index) in tasks.iter_mut(){
        transform.translation = teleport_task.loc;
        commands.entity(task_entity).remove::<TeleportTask>();
        jobs.next_task(&mut commands, &task_entity, job_index);
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
    mut jobs:       ResMut<Jobs>,
    time:           Res<Time>,
    mut tasks:      Query<(Entity, &mut WaitTask, &JobIndex)>,
){

    for (task_entity, mut wait_task, job_index) in tasks.iter_mut(){
        match &mut wait_task.schedule {
            JobSchedule::RealDelay(delay) => {
                if *delay > 0.0 {
                    *delay -= time.delta_secs();
                } else {
                    commands.entity(task_entity).remove::<WaitTask>();
                    jobs.next_task(&mut commands, &task_entity, job_index);
                }
            }
            _ => {}
        }
    }
}


pub fn wait_idle_calendar(
    mut commands: Commands,
    mut jobs:     ResMut<Jobs>,
    calendar:     Res<Calendar>,
    mut idles:    Query<(Entity, &mut WaitTask, &JobIndex)>
){

    for (task_entity, mut wait_task, job_index) in idles.iter_mut(){
        match &mut wait_task.schedule {
                JobSchedule::Cron(cron) => {
                    if cron.is_time(&calendar){
                        commands.entity(task_entity).remove::<WaitTask>();
                        jobs.next_task(&mut commands, &task_entity, job_index);

                    }
                 }
                 JobSchedule::Delay(delay) => {
                    if *delay > 0 {
                        *delay -= 1;
                    } else {
                        commands.entity(task_entity).remove::<WaitTask>();
                        jobs.next_task(&mut commands, &task_entity, job_index);
                    }
                }
                _=> {}   
        }
    }
}
