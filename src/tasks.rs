use bevy::prelude::*;
use crate::utils::{get_direction, get_distance_manhattan, get_random_range_u32, move_x, move_y};
use serde::{Deserialize, Serialize};
use crate::pg_jobs::{Jobs, JobSchedule, JobCatalog, JobID};
use bevy_pg_calendar::prelude::Calendar;

const TASK_DEBUG: bool = false;
macro_rules! tdbg {
    ($a:expr)=>{
        {if TASK_DEBUG {info!(" [AI] {}", $a)};}
    }
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct SpawnTask {
    pub color:  Color,
    pub loc:    Vec3
}

#[derive(Component, Clone, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct SpawnGroupTask {
    pub data:  Vec<JobID>
}


#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct DespawnTask;

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct MoveTask {
    pub source:         Vec3,
    pub target:         Vec3,
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct RotateTask {
    pub angle:      f32
}

#[derive(Component, Clone, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct WaitTask {
    pub schedule: JobSchedule
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct HideTask;

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct ShowTask;

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct TeleportTask {
    pub loc: Vec3
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct DecisionTask {
    pub opt1: u32,
    pub opt2: u32
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct LoopTask {
    pub start_id:  u32, // Loops the tasks specified in the vector
    pub maxk:      Option<u32>
}
impl Default for LoopTask {
    fn default() -> Self {
        LoopTask{start_id: 0, maxk: None}
    }
}

/* TASK SYSTEMS */

pub fn spawn_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    tasks:          Query<(Entity, &SpawnTask)>,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<ColorMaterial>>,
){

    for (task_entity, spawn_task) in tasks.iter(){

        commands.entity(task_entity).insert((
            Mesh2d(meshes.add(Rectangle::from_size(Vec2 { x: 100.0, y: 100.0 })).into()),
            Transform::from_translation(spawn_task.loc),
            MeshMaterial2d(materials.add(spawn_task.color))
        ));

        commands.entity(task_entity).remove::<SpawnTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}


pub fn wait_task_time(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    time:           Res<Time>,
    mut tasks:      Query<(Entity, &mut WaitTask)>,
){

    for (task_entity, mut wait_task) in tasks.iter_mut(){

        match &mut wait_task.schedule {
            JobSchedule::RealDelay(delay) => {
                if *delay > 0.0 {
                    *delay -= time.delta_secs();
                } else {
                    commands.entity(task_entity).remove::<WaitTask>();
                    jobs.next_task(&mut commands, &task_entity);
                }
            }
            _ => {}
        }
    }
}


pub fn wait_idle_calendar(
    mut commands:     Commands,
    mut jobs:         ResMut<Jobs>,
    calendar:         Res<Calendar>,
    mut idle_cars:    Query<(Entity, &mut WaitTask)>
){

    for (task_entity, mut wait_task) in idle_cars.iter_mut(){
        match &mut wait_task.schedule {
                JobSchedule::Cron(cron) => {
                    if cron.hours.as_ref().unwrap().contains(&calendar.get_current_hour()) && 
                       cron.days_week.as_ref().unwrap().contains(&calendar.get_current_weekday()){

                        commands.entity(task_entity).remove::<WaitTask>();
                        jobs.next_task(&mut commands, &task_entity);

                    }
                 }
                 JobSchedule::Delay(delay) => {
                    if *delay > 0 {
                        *delay -= 1;
                    } else {
                        commands.entity(task_entity).remove::<WaitTask>();
                        jobs.next_task(&mut commands, &task_entity);
                    }
                }
                _=> {}   
        }
    }

}

pub fn move_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    time:           Res<Time>,
    mut tasks:      Query<(Entity, &mut Transform, &mut MoveTask)>,
){

    let speed = 200.0;
    for (task_entity, mut transform, move_task) in tasks.iter_mut(){

        let angle: f32 = get_direction(&transform.translation.xy(), &move_task.target.xy());
        let dist: f32 = get_distance_manhattan(&transform.translation.xy(), &move_task.target.xy());
        let local_speed = speed*time.delta_secs();
        if local_speed > dist {
            commands.entity(task_entity).remove::<MoveTask>();
            jobs.next_task(&mut commands, &task_entity);
        } else {
            transform.translation.x += move_x(local_speed, angle);
            transform.translation.y += move_y(local_speed, angle);    
        }

    }

}

pub fn rotate_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    mut tasks:      Query<(Entity, &mut Transform, &RotateTask)>,
){

    let ang_speed = 0.05;
    for (task_entity, mut transform, rotate_task) in tasks.iter_mut(){
        let angle: f32 = transform.rotation.to_euler(EulerRot::XYZ).2.to_degrees();
        if angle < rotate_task.angle {
            transform.rotate_z(ang_speed);
        } else {
            commands.entity(task_entity).remove::<RotateTask>();
            jobs.next_task(&mut commands, &task_entity);
        }
    }
}

pub fn teleport_task(mut commands:   Commands,
                 mut jobs:       ResMut<Jobs>,
                 mut tasks:      Query<(Entity, &mut Transform, &TeleportTask)>){

    for (task_entity, mut transform, teleport_task) in tasks.iter_mut(){
        transform.translation = teleport_task.loc;
        commands.entity(task_entity).remove::<TeleportTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
    
}

pub fn show_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             mut tasks:      Query<(Entity, &mut Visibility), With<ShowTask>>){

    for (task_entity, mut vis) in tasks.iter_mut(){
        *vis = Visibility::Inherited;
        commands.entity(task_entity).remove::<ShowTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}

pub fn hide_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             mut tasks:      Query<(Entity, &mut Visibility), With<HideTask>>){

    for (task_entity, mut vis) in tasks.iter_mut(){
        *vis = Visibility::Hidden;
        commands.entity(task_entity).remove::<HideTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}

// Example Decision task
pub fn decision_task(mut commands:   Commands,
                 mut jobs:       ResMut<Jobs>,
                 tasks:          Query<(Entity, &DecisionTask)>){

    for (task_entity, decision_task) in tasks.iter(){
        let random_value = get_random_range_u32(0, 100);
        let next_task_id: u32;
        if random_value <= 50 {
            next_task_id = decision_task.opt1;
        } else {
            next_task_id = decision_task.opt2;
        }
        commands.entity(task_entity).remove::<DecisionTask>();
        jobs.jump_task(&mut commands, &task_entity, next_task_id);
    }

}

pub fn loop_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             tasks:          Query<(Entity, &LoopTask)>){

    for (task_entity, loop_task) in tasks.iter(){

        if let Some(maxk) = loop_task.maxk {
            if let Some(job) = jobs.get_mut(&task_entity){
                // final iteration
                if job.loopk()>= maxk {
                    job.loop_reset();
                    commands.entity(task_entity).remove::<LoopTask>();
                    jobs.next_task(&mut commands, &task_entity); 
                } else {
                    job.loop_incr();
                    commands.entity(task_entity).remove::<LoopTask>();
                    jobs.jump_task(&mut commands, &task_entity, loop_task.start_id); 
                }
            }
        }
    }
}

pub fn despawn_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    tasks:          Query<Entity, With<DespawnTask>>){

    for task_entity in tasks.iter(){
        commands.entity(task_entity).despawn();
        jobs.remove_all(&task_entity);
    }

}



/* 
// Invented the extendable task -> Replace task structs but keep the same task in job.

fn move_to_chair_task(
    mut commands:      Commands,
    mut tables:        Query<(Entity, &mut Table, &Relatives)>,
    mut chairs:        Query<(Entity, &Transform, &mut Chair)>,
    mut tasks:         Query<(Entity, &Transform, &mut AnimData), With<MoveToChairTask>>){

    for (task_entity, task_transform, mut anim_data) in tasks.iter_mut(){

        for (_table_entity, mut table, table_chairs) in tables.iter_mut(){
            if !table.free {
                continue; // need free table
            }
            let mut found_sit: bool = false;
            for chair_entity in table_chairs.data.iter(){
    
                if let Ok((chair_entity, chair_transform, mut chair)) = chairs.get_mut(*chair_entity) {
                    if !chair.free {
                        continue; // need free chair
                    }
                    let mobj: MoveTask = MoveTask{
                        source: task_transform.translation, 
                        target: chair_transform.translation,
                        route: None,
                        route_index: 0
                    };
                    anim_data.set(AnimType::Walk);
                    commands.entity(task_entity).insert(mobj);
                    commands.entity(task_entity).remove::<MoveToChairTask>();
                    commands.entity(task_entity).insert(PickedChair::new(chair.angle, chair_entity));
                    chair.free = false;
                    table.free = false;
                    found_sit = true;
                    
                    break;
                }
            }
            if found_sit {
                break;
            }
        }
    }
}

*/


// Pattern of spawning the group from task
pub fn spawn_group_task(
    mut commands:   Commands,
    tasks:          Query<(Entity, &SpawnGroupTask)>,
    mut jobs:       ResMut<Jobs>,
    jobcatalog:     Res<JobCatalog>
){

    for (task_entity, spawn_group_task) in tasks.iter(){ 
        
        for (index, _next_job) in spawn_group_task.data.iter().enumerate(){
            // instead of spawn_with_task:
            let next_job = spawn_group_task.data[index];
            jobcatalog.start(&mut commands, next_job, &mut jobs);       
        }
        commands.entity(task_entity).remove::<SpawnGroupTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}
