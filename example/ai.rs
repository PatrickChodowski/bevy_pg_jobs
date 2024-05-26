use bevy::prelude::*;
use bevy_pg_jobs::prelude::{Job, JobSchedule, JobTasks, Jobs, PGJobsPlugin, TasksPlugin, DESPAWN_TASK_ID, SPAWN_TASK_ID};
use bevy_pg_jobs::tasks::{TaskType, SpawnTask, DespawnTask, RotateTask, MoveTask, 
    WaitTask, TeleportTask, ShowTask, HideTask};

pub struct AIPlugin;

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
    let end_loc = Vec3::new(300.0, 300.0, 1.0);

    tasks.add(SPAWN_TASK_ID, TaskType::Spawn(SpawnTask{loc: start_loc, color: Color::RED}));
    tasks.add(1,   TaskType::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)}));
    tasks.add(2,   TaskType::Move(MoveTask{source: start_loc, target: end_loc}));
    tasks.add(3,   TaskType::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)}));
    tasks.add(4,   TaskType::Rotate(RotateTask{angle: 90.0}));
    tasks.add(5,   TaskType::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)}));
    tasks.add(6,   TaskType::Teleport(TeleportTask{loc: start_loc}));
    tasks.add(7,   TaskType::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)}));
    tasks.add(8,   TaskType::Teleport(TeleportTask{loc: end_loc}));
    tasks.add(9,   TaskType::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)}));
    tasks.add(10,  TaskType::Hide(HideTask));
    tasks.add(11,  TaskType::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)}));
    tasks.add(12,  TaskType::Show(ShowTask));
    tasks.add(13,  TaskType::Wait(WaitTask{schedule: JobSchedule::RealDelay(delay)}));
    tasks.add(DESPAWN_TASK_ID, TaskType::Despawn(DespawnTask));

    let test_job = Job{tasks, ..default()};
    jobs.add(test_job);
}
