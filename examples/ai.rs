use bevy::prelude::*;
use bevy_pg_jobs::prelude::{Job, JobSchedule, JobTasks, Jobs, PGJobsPlugin, 
    TaskData, TaskStatus, TasksPlugin, DESPAWN_TASK_ID, SPAWN_TASK_ID};
use bevy_pg_jobs::tasks::{Task, SpawnTask, DespawnTask, RotateTask, MoveTask, 
    WaitTask, TeleportTask, ShowTask, HideTask, DecisionTask, LoopTask};

pub struct AIPlugin;

pub const TEST_JOB: u32 = 0;
pub const FIRETRUCK_JOB: u32 = 1;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app 
        .add_plugins(PGJobsPlugin::default())
        .add_plugins(TasksPlugin) 
        .add_systems(Startup, init)
        ;
    }
}

fn init(mut jobs: ResMut<Jobs>){
    let mut tasks = JobTasks::new();
    let delay: f32 = 0.5;
    let start_loc = Vec3::new(100.0, 100.0, 1.0);
    let end_loc1 = Vec3::new(300.0, 300.0, 1.0);
    let end_loc2 = Vec3::new(-300.0, -300.0, 1.0);

    tasks.add(TaskData{id:     SPAWN_TASK_ID, 
                       status: TaskStatus::ToDo, 
                       task:   Task::Spawn(SpawnTask{loc: start_loc, color: Color::RED}), 
                       ..default()});

    tasks.add(TaskData::new(1, 100,  Task::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)})));
    tasks.add(TaskData::idt(100,     Task::Decision(DecisionTask{opt1: 20, opt2: 21})));
    tasks.add(TaskData::new(20, 22,  Task::Move(MoveTask{source: start_loc, target: end_loc1})));
    tasks.add(TaskData::new(21, 22,  Task::Move(MoveTask{source: start_loc, target: end_loc2})));
    tasks.add(TaskData::new(22, 3,   Task::Loop(LoopTask{start_id: 100, maxk: Some(3), ..default()})));
    tasks.add(TaskData::idt(3,   Task::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)})));
    tasks.add(TaskData::idt(4,   Task::Rotate(RotateTask{angle: 90.0})));
    tasks.add(TaskData::idt(5,   Task::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)})));
    tasks.add(TaskData::idt(6,   Task::Teleport(TeleportTask{loc: start_loc})));
    tasks.add(TaskData::idt(7,   Task::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)})));
    tasks.add(TaskData::idt(8,   Task::Teleport(TeleportTask{loc: end_loc1})));
    tasks.add(TaskData::idt(9,   Task::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)})));
    tasks.add(TaskData::idt(10,  Task::Hide(HideTask)));
    tasks.add(TaskData::idt(11,  Task::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)})));
    tasks.add(TaskData::idt(12,  Task::Show(ShowTask)));
    tasks.add(TaskData::new(13, DESPAWN_TASK_ID,  Task::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)})));
    tasks.add(TaskData::idt(DESPAWN_TASK_ID, Task::Despawn(DespawnTask)));

    let test_job = Job{tasks, id: TEST_JOB, ..default()};
    jobs.add(test_job);
}



/* Dynamic jobs Example: pass arguments to a function and trigger a job by creating it dynamically from system*/
fn firetruck_job(start_loc: Vec3, color: Color) -> Job {
    let mut tasks = JobTasks::new();
    tasks.add(TaskData{id:     SPAWN_TASK_ID, 
                       status: TaskStatus::ToDo, 
                       task:   Task::Spawn(SpawnTask{loc: start_loc, color}), ..default()});

    let job = Job{tasks, id: FIRETRUCK_JOB, ..default()};
    return job;
}
