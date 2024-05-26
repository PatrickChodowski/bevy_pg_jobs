
/*
    This script is to be added to the main game app. It should contain:
    - Tasks Plugin
    - TaskType enum
    - WorkerTask structs for each Task
    - systems to dispatch tasks
    - definition of JobTask
    - TaskStatus enum
*/

use bevy::ecs::storage::Table;
use bevy::ecs::world::World;
use bevy::ecs::system::Resource;
use bevy::ecs::entity::Entity;
use bevy::app::{App, Plugin, Startup, Update};
use bevy::transform::commands;
use bevy::utils::HashMap;
use bevy::ecs::schedule::{IntoSystemConfigs, Condition};
use bevy::log::info;
use bevy::ecs::system::{RunSystemOnce, SystemId, Commands, ResMut, Res, Query};
use bevy::ecs::component::{Component, ComponentStorage, TableStorage, SparseStorage};
use bevy::ecs::bundle::{Bundle, DynamicBundle};

pub struct TasksPlugin;

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Update, update_tasks)
        ;
    }
}

fn update_tasks(){

}


// Fill it up with the tasks for the game
#[derive(Component, Clone, Copy)]
pub enum TaskType {
    Spawn(SpawnTask),
    Despawn(DespawnTask),
    Move(MoveTask),
    Rotate(RotateTask),
    Wait(WaitTask)
}
impl TaskType {
    pub fn spawn_with_task(&self, commands: &mut Commands) -> Entity {
        match &self {
            TaskType::Spawn(data)   => {commands.spawn(*data).id()}
            TaskType::Despawn(data) => {commands.spawn(*data).id()}
            TaskType::Move(data)    => {commands.spawn(*data).id()}
            TaskType::Rotate(data)  => {commands.spawn(*data).id()}
            TaskType::Wait(data)    => {commands.spawn(*data).id()}
        }
    }
}



// JobTasks
pub struct JobTasks {
    pub data:                   HashMap<u32, TaskType>,
    pub current_task_id:        u32,
    pub current_task_status:    TaskStatus
}

impl JobTasks {
    pub fn new() -> Self {
        JobTasks{
            data:                   HashMap::new(),
            current_task_id:        0,
            current_task_status:    TaskStatus::ToDo
        }
    }
    pub fn add(&mut self, id: u32, task: TaskType) {
        self.data.insert(id, task);
    }
    pub fn get_current(&self) -> &TaskType {
        &self.data[&self.current_task_id]
    }
    pub fn set_current_active(&mut self) {
        self.current_task_status = TaskStatus::Active;
    }
}


/* 
impl JobTasks {
    pub fn new(tasks: &Vec<Task>, fails: &Option<Vec<Task>>) -> Self {
        let mut tsks = Tasks{data: tasks.clone(), current: 0, pause: false};
        tsks.data[0].task_status = TaskStatus::ToDo;
        return tsks;
    }

    pub fn new_after_spawn(tasks: &Vec<Task>, fails: &Option<Vec<Task>>) -> Self {
        let mut tsks = Tasks{data: tasks.clone(), current: 1, pause: false};
        tsks.data[0].task_status = TaskStatus::Done;
        tsks.data[1].task_status = TaskStatus::ToDo;
        return tsks;
    }

    pub fn new_without_spawn(tasks: &Vec<Task>, fails: &Option<Vec<Task>>) -> Self {
        let mut tsks = Tasks{data: tasks.clone(), current: 0, pause: false};
        tsks.data[0].task_status = TaskStatus::ToDo;
        return tsks;
    }

    pub fn set_current_active(&mut self) {
        self.data[self.current].task_status = TaskStatus::Active;
    }
    pub fn pause(&mut self){
        self.pause = true;
    }
    pub fn unpause(&mut self){
        self.pause = false;
    }

    pub fn set_current_loop(&mut self, steps: usize) {
        if self.data[self.current].task_status == TaskStatus::Done {
            return; // For the forever loops
        }

        if let TaskType::LoopLastNStepsKTimes((_n, k)) = &mut self.data[self.current].task_type {
            *k -= 1;
        }

        self.data[self.current].task_status = TaskStatus::InLoop(steps);
        
    }

    pub fn fail_task(&mut self){
        self.data[self.current].task_status = TaskStatus::Fail;
        if let Some(fails) = &self.fails {
            self.data = fails.to_vec();
            self.current = 0;
            self.data[self.current].task_status = TaskStatus::ToDo;
        } else {
            self.data = vec![Task::despawn_task()];
            self.current = 0;
            self.data[self.current].task_status = TaskStatus::ToDo;
        }
    }
    pub fn next_task_close_loop(&mut self) {

        let mut next_loop_index: Option<usize> = None;
        for (index, task) in self.data.iter().enumerate(){
            if index <= self.current {
                continue; // not important anymore
            }
            if !task.task_type.is_loop() {
                continue; // Searching for next loop
            }
            next_loop_index = Some(index);
            break;
        }
        if let Some(loop_index) = next_loop_index {
            self.current = loop_index;
            self.data[self.current].task_status = TaskStatus::Done;
        } else {
            panic!("Asks for closing loop, but there is no loop to close?");
        }
    }

    pub fn next_task(&mut self) {
        let current_task = &self.data[self.current];
        match current_task.task_status {
            TaskStatus::Done => {
                // Should be only if loop was requested to close
                self.current += 1;
                self.data[self.current].task_status = TaskStatus::ToDo;
            }
            TaskStatus::Active => {
                self.data[self.current].task_status = TaskStatus::Done;
                self.current += 1;
                self.data[self.current].task_status = TaskStatus::ToDo;
            }
            TaskStatus::InLoop(steps) => {
                for i in 0..=steps {
                    self.data[self.current-i].task_status = TaskStatus::Waiting;
                }
                self.current -= steps;
                self.data[self.current].task_status = TaskStatus::ToDo;
            }
            TaskStatus::ToDo => {
                // When the loop task finished
                self.data[self.current].task_status = TaskStatus::Done;
                self.current += 1;
                self.data[self.current].task_status = TaskStatus::ToDo;
            }
            _ => {panic!("Not supposed to happen {:?}", current_task )}
        }
    }
}
*/






#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    ToDo,
    Waiting,
    Active,
    InLoop(usize), // Number of task steps to repeat every time
    Done,
    Fail
}


// Define component for each task

#[derive(Component, Clone, Copy)]
pub struct SpawnTask;

#[derive(Component, Clone, Copy)]
pub struct DespawnTask;

#[derive(Component, Clone, Copy)]
pub struct MoveTask;

#[derive(Component, Clone, Copy)]
pub struct RotateTask;

#[derive(Component, Clone, Copy)]
pub struct WaitTask;

