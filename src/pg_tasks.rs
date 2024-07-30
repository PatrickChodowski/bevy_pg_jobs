// Bevy dependencies
use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize, Deserializer, de::Error, de::Unexpected};


use bevy_pg_calendar::prelude::CalendarNewHourEvent;
use crate::tasks::*;

pub const SPAWN_TASK_ID:   u32 = 0;
pub const DESPAWN_TASK_ID: u32 = 1000;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TaskSets {
    Dispatch,
    Extension,
    Simple,
    Loop,
    Decision
}


pub struct TasksPlugin;

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app
        .configure_sets(Update, (
            TaskSets::Dispatch, 
            TaskSets::Extension, 
            TaskSets::Simple, 
            TaskSets::Decision,
            TaskSets::Loop
        ).chain())
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

    pub fn remove(&self, commands: &mut Commands, entity: &Entity){
        match &self {
            Task::Spawn(_data)           => {commands.entity(*entity).remove::<SpawnTask>();}
            Task::Despawn(_data)         => {commands.entity(*entity).remove::<DespawnTask>();}
            Task::Move(_data)            => {commands.entity(*entity).remove::<MoveTask>();}
            Task::Wait(_data)            => {commands.entity(*entity).remove::<WaitTask>();}
            Task::Hide(_data)            => {commands.entity(*entity).remove::<HideTask>();}
            Task::Show(_data)            => {commands.entity(*entity).remove::<ShowTask>();}
            Task::Teleport(_data)        => {commands.entity(*entity).remove::<TeleportTask>();}
            Task::Decision(_data)        => {commands.entity(*entity).remove::<DecisionTask>();}
            Task::Loop(_data)            => {commands.entity(*entity).remove::<LoopTask>();}
            Task::Rotate(_data)          => {commands.entity(*entity).remove::<RotateTask>();}
        }
    }
}

// TaskData
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TaskData {
    #[serde(skip_deserializing)]
    pub id:     u32,
    pub next:   Option<u32>,
    pub task:   Task  
}
impl Default for TaskData {
    fn default() -> Self {
        TaskData{
            id: SPAWN_TASK_ID, 
            next: None, 
            task: Task::Despawn(DespawnTask)
        }
    }
}

impl TaskData {
    pub fn idtn(id: u32, next: u32, task: Task) -> Self {
        TaskData{id, next: Some(next), task}
    }
    pub fn idt(id: u32, task: Task) -> Self {
        TaskData{id, task, ..default()}
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobTasks {
    #[serde(deserialize_with = "deserialize_jobtask_data")]
    pub data:                   HashMap<u32, TaskData>,  
    #[serde(skip)] 
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
            info!(" [Tasks]: Starting job for entity: {:?}", job_entity);
            return job_entity;
        } else {
            let entity = current_task.task.spawn_with_task(commands);
            info!(" [Tasks]: Spawning job entity: {:?}", entity);
            return entity;
        }
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
        if let Some(task_data) = self.data.get(&self.current_task_id) {
            return &task_data.task;
        } else {
            panic!("no task for {}", self.current_task_id);
        }
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

