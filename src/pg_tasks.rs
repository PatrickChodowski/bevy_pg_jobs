// Bevy dependencies
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::sprite::MaterialMesh2dBundle;
use serde::{Deserialize, Serialize, Deserializer, de::Error, de::Unexpected};
use crate::utils::{get_direction, get_distance_manhattan, get_random_range_u32, move_x, move_y};

use bevy_pg_calendar::prelude::{Calendar, CalendarNewHourEvent};
use crate::pg_jobs::{Jobs, JobSchedule, JobCatalog};

pub const SPAWN_TASK_ID:   u32 = 0;
pub const DESPAWN_TASK_ID: u32 = 1000;

pub struct TasksPlugin;

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Update, ((spawn_group_task, spawn_task).chain(), 
                              wait_task_time, 
                              wait_idle_calendar.run_if(on_event::<CalendarNewHourEvent>()),
                              move_task,
                              rotate_task,
                              teleport_task,
                              hide_task,
                              show_task,
                              decision_task,
                              despawn_task,
                              loop_task
                            ))
        ;
    }
}

// Fill it up with the tasks for the game
#[derive(Component, Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Task {
    Spawn(SpawnTask),
    Despawn(DespawnTask),
    Move(MoveTask),
    Rotate(RotateTask),
    Wait(WaitTask),
    Hide(HideTask),
    Show(ShowTask),
    Teleport(TeleportTask),
    Decision(DecisionTask),
    Loop(LoopTask)          // Loops specified tasks till broken by decision task
}
impl Task {
    pub fn spawn_with_task(&self, commands: &mut Commands) -> Entity {
        match &self {
            Task::Spawn(data)       => {commands.spawn(*data).id()}
            Task::Despawn(data)     => {commands.spawn(*data).id()}
            Task::Move(data)        => {commands.spawn(*data).id()}
            Task::Rotate(data)      => {commands.spawn(*data).id()}
            Task::Wait(data)        => {commands.spawn(data.clone()).id()}
            Task::Hide(data)        => {commands.spawn(*data).id()}
            Task::Show(data)        => {commands.spawn(*data).id()}
            Task::Teleport(data)    => {commands.spawn(*data).id()}
            Task::Decision(data)    => {commands.spawn(*data).id()}
            Task::Loop(data)        => {commands.spawn(*data).id()}
        }
    }
    pub fn add_task(&self, commands: &mut Commands, entity: &Entity) {
        match &self {
            Task::Spawn(data)    => {commands.entity(*entity).insert(*data);}
            Task::Despawn(data)  => {commands.entity(*entity).insert(*data);}
            Task::Move(data)     => {commands.entity(*entity).insert(*data);}
            Task::Rotate(data)   => {commands.entity(*entity).insert(*data);}
            Task::Wait(data)     => {commands.entity(*entity).insert(data.clone());}
            Task::Hide(data)     => {commands.entity(*entity).insert(*data);}
            Task::Show(data)     => {commands.entity(*entity).insert(*data);}
            Task::Teleport(data) => {commands.entity(*entity).insert(*data);}
            Task::Decision(data) => {commands.entity(*entity).insert(*data);}
            Task::Loop(data)     => {commands.entity(*entity).insert(*data);}
        }
    }
    pub fn display(&self) -> String {
        match &self {
            Task::Spawn(data)    => {format!("Spawn: loc: {}, color: {:?}", data.loc, data.color)}
            Task::Despawn(_data) => {format!("Despawn")}
            Task::Move(data)     => {format!("Move: from: {}, to: {}", data.source, data.target)}
            Task::Rotate(data)   => {format!("Rotate: angle: {}", data.angle)}
            Task::Wait(data)     => {format!("Wait: schedule: {:?}", data.schedule)}
            Task::Hide(_data)    => {format!("Hide")}
            Task::Show(_data)    => {format!("Show")}
            Task::Teleport(data) => {format!("Teleport: to: {}", data.loc)}
            Task::Decision(data) => {format!("Decision: opt1: {}, opt2: {}", data.opt1, data.opt2)}
            Task::Loop(data)     => {format!("Loop: start_id: {}, maxk: {:?}", data.start_id, data.maxk)}
        }
    }

    pub fn remove(&self, commands: &mut Commands, entity: Entity){
        match &self {
            Task::Spawn(_data)           => {commands.entity(entity).remove::<SpawnTask>();}
            Task::Despawn(_data)         => {commands.entity(entity).remove::<DespawnTask>();}
            Task::Move(_data)            => {commands.entity(entity).remove::<MoveTask>();}
            Task::Wait(_data)            => {commands.entity(entity).remove::<WaitTask>();}
            Task::Hide(_data)            => {commands.entity(entity).remove::<HideTask>();}
            Task::Show(_data)            => {commands.entity(entity).remove::<ShowTask>();}
            Task::Teleport(_data)        => {commands.entity(entity).remove::<TeleportTask>();}
            Task::Decision(_data)        => {commands.entity(entity).remove::<DecisionTask>();}
            Task::Loop(_data)            => {commands.entity(entity).remove::<LoopTask>();}
            Task::Rotate(_data)          => {commands.entity(entity).remove::<RotateTask>();}
        }
    }
}

// TaskData
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TaskData {
    #[serde(skip_deserializing)]
    pub id:     u32,
    pub next:   Option<u32>,
    #[serde(default)]
    pub status: TaskStatus,
    pub task:   Task  
}
impl Default for TaskData {
    fn default() -> Self {
        TaskData{
            id: SPAWN_TASK_ID, 
            next: None, 
            status: TaskStatus::Waiting, 
            task: Task::Despawn(DespawnTask)
        }
    }
}

impl TaskData {
    pub fn new(id: u32, next: u32, task: Task) -> Self {
        TaskData{id, next: Some(next), task, status: TaskStatus::Waiting}
    }
    pub fn idt(id: u32, task: Task) -> Self {
        TaskData{id, task, ..default()}
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobTasks {
    #[serde(deserialize_with = "deserialize_jobtask_data")]
    pub data:                   HashMap<u32, TaskData>,   
    pub current_task_id:        u32,
}

impl JobTasks { pub fn new() -> Self {
        JobTasks{
            data:                   HashMap::new(),
            current_task_id:        0,
        }
    }

    pub fn add(&mut self, task_data: TaskData) {
        self.data.insert(task_data.id, task_data);
    }

    pub fn get_next_id(&mut self) -> u32 {
        let current_task_data = self.data.get(&self.current_task_id).unwrap();
        if let Some(next_task_id) = current_task_data.next {
            return next_task_id;
        } else {
            return current_task_data.id + 1;
        }
    }

    pub fn start(&mut self, commands: &mut Commands, job_entity: Option<Entity>) -> Entity {
        let current_task = &self.data.get(&self.current_task_id).unwrap();
        if let Some(job_entity) = job_entity {
            current_task.task.add_task(commands, &job_entity);
            self.set_current_status(TaskStatus::Active);
            info!(" [Tasks]: Starting job for entity: {:?}", job_entity);
            return job_entity;
        } else {
            let entity = current_task.task.spawn_with_task(commands);
            self.set_current_status(TaskStatus::Active);
            info!(" [Tasks]: Spawning job entity: {:?}", entity);
            return entity;
        }
    }

    pub fn set_task(&mut self, next_task_id: u32) -> &Task {
        self.current_task_id = next_task_id;
        self.set_current_status(TaskStatus::ToDo);
        return self.get_current();
    }
    
    pub fn next_task(&mut self) -> &Task {
        match self.get_current_status() {
            &TaskStatus::Done => {
                // Should be only if loop was requested to close
                self.current_task_id = self.get_next_id();
                self.set_current_status(TaskStatus::ToDo);
                return self.get_current();
            }
            &TaskStatus::Active => {
                self.set_current_status(TaskStatus::Done);
                self.current_task_id = self.get_next_id();
                self.set_current_status(TaskStatus::ToDo);
                return self.get_current();
            }
            &TaskStatus::ToDo => {
                // When the loop task finished
                self.set_current_status(TaskStatus::Done);
                self.current_task_id = self.get_next_id();
                self.set_current_status(TaskStatus::ToDo);
                return self.get_current();
            }
            _ => {
                panic!("Not supposed to happen {:?}", self.current_task_id )
            }
        }
    }
    pub fn get_current(&self) -> &Task {
        if let Some(task_data) = self.data.get(&self.current_task_id) {
            return &task_data.task;
        } else {
            panic!("no task for {}", self.current_task_id);
        }
    }
    pub fn set_current_status(&mut self, status: TaskStatus) {
        if let Some(task_data) = self.data.get_mut(&self.current_task_id){
            task_data.status = status;
        }
    }
    pub fn get_current_status(&mut self) -> &TaskStatus {
        &self.data.get(&self.current_task_id).unwrap().status
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]

pub enum TaskStatus {
    ToDo,
    #[default]
    Waiting,
    Active,
    Done,
    Fail
}


/* TASK STRUCTS */

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct SpawnTask {
    pub color:  Color,
    pub loc:    Vec3
}

#[derive(Component, Clone, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct SpawnGroupTask {
    pub data:  Vec<u32>
}


#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct DespawnTask;

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct MoveTask {
    pub source:         Vec3,
    pub target:         Vec3,
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct RotateTask {
    pub angle:      f32
}

#[derive(Component, Clone, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct WaitTask {
    pub schedule: JobSchedule
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct HideTask;

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct ShowTask;

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct TeleportTask {
    pub loc: Vec3
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
#[component(storage = "SparseSet")]
pub struct DecisionTask {
    pub opt1: u32,
    pub opt2: u32
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
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

fn spawn_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    tasks:          Query<(Entity, &SpawnTask)>,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<ColorMaterial>>,
){

    for (task_entity, spawn_task) in tasks.iter(){

        commands.entity(task_entity).insert(
            MaterialMesh2dBundle {
                mesh: meshes.add(Rectangle::from_size(Vec2 { x: 100.0, y: 100.0 })).into(),
                transform: Transform::from_translation(spawn_task.loc),
                material: materials.add(spawn_task.color),
                ..default()}
        );

        commands.entity(task_entity).remove::<SpawnTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}


fn wait_task_time(mut commands:   Commands,
                  mut jobs:       ResMut<Jobs>,
                  time:           Res<Time>,
                  mut tasks:      Query<(Entity, &mut WaitTask)>,){

    for (task_entity, mut wait_task) in tasks.iter_mut(){

        match &mut wait_task.schedule {
            JobSchedule::RealDelay(delay) => {
                if *delay > 0.0 {
                    *delay -= time.delta_seconds();
                } else {
                    commands.entity(task_entity).remove::<WaitTask>();
                    jobs.next_task(&mut commands, &task_entity);
                }
            }
            _ => {}
        }
    }
}


pub fn wait_idle_calendar(mut commands:     Commands,
                          mut jobs:         ResMut<Jobs>,
                          calendar:         Res<Calendar>,
                          mut idle_cars:    Query<(Entity, &mut WaitTask)>){

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

fn move_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             time:           Res<Time>,
             mut tasks:      Query<(Entity, &mut Transform, &mut MoveTask)>,){

    let speed = 200.0;
    for (task_entity, mut transform, move_task) in tasks.iter_mut(){

        let angle: f32 = get_direction(&transform.translation.xy(), &move_task.target.xy());
        let dist: f32 = get_distance_manhattan(&transform.translation.xy(), &move_task.target.xy());
        let local_speed = speed*time.delta_seconds();
        if local_speed > dist {
            commands.entity(task_entity).remove::<MoveTask>();
            jobs.next_task(&mut commands, &task_entity);
        } else {
            transform.translation.x += move_x(local_speed, angle);
            transform.translation.y += move_y(local_speed, angle);    
        }

    }

}

fn rotate_task(mut commands:   Commands,
               mut jobs:       ResMut<Jobs>,
               mut tasks:      Query<(Entity, &mut Transform, &RotateTask)>,){

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

fn teleport_task(mut commands:   Commands,
                 mut jobs:       ResMut<Jobs>,
                 mut tasks:      Query<(Entity, &mut Transform, &TeleportTask)>){

    for (task_entity, mut transform, teleport_task) in tasks.iter_mut(){
        transform.translation = teleport_task.loc;
        commands.entity(task_entity).remove::<TeleportTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
    
}

fn show_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             mut tasks:      Query<(Entity, &mut Visibility), With<ShowTask>>){

    for (task_entity, mut vis) in tasks.iter_mut(){
        *vis = Visibility::Inherited;
        commands.entity(task_entity).remove::<ShowTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}

fn hide_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             mut tasks:      Query<(Entity, &mut Visibility), With<HideTask>>){

    for (task_entity, mut vis) in tasks.iter_mut(){
        *vis = Visibility::Hidden;
        commands.entity(task_entity).remove::<HideTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}

// Example Decision task
fn decision_task(mut commands:   Commands,
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

fn loop_task(mut commands:   Commands,
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

fn despawn_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    tasks:          Query<Entity, With<DespawnTask>>){

    for task_entity in tasks.iter(){
        commands.entity(task_entity).despawn_recursive();
        jobs.remove_all(&task_entity);
    }

}



// Converts the type of ID to int and also updates the ID value
fn deserialize_jobtask_data<'de, D>(deserializer: D) -> Result<HashMap<u32, TaskData>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_map = HashMap::<String, TaskData>::deserialize(deserializer)?;
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
