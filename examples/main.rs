use bevy::prelude::*;
use bevy_pg_jobs::prelude::*;
use bevy_pg_calendar::prelude::PGCalendarPlugin;
use bevy_pg_jobs::common::*;
use bevy_pg_jobs::macros::{first, next};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PGCalendarPlugin{
                active:      false,
                hour_length: 5,
                start_hour:  6,
                ..default()
            },
            PGJobsPlugin::default()
        ))
        .register_type::<Player>()
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.1)))
        .add_systems(Startup, init)
        .add_systems(Update, (
            wait_task_time,
            hide_task, 
            show_task,
            despawn_task
        ))
        .run();
}


fn init(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
){
    commands.spawn(Camera2d);

    let square_sprite = Sprite {
        color: Color::srgb(0.7, 0.7, 0.8),
        custom_size: Some(Vec2::splat(50.0)),
        ..default()
    };
    let mut jt = JobTasks::new();
    first!(jt, HideTask);
    next!(jt, ShowTask);
    first!(jt, HideTask, 0);
    next!(jt, ShowTask, 0);

    commands.spawn((
        square_sprite.clone(),
        Transform::from_xyz(0.0, 50.0 * 6.0, 0.0).with_scale(Vec3::new(20.0, 1.0, 1.0)),
    ));
    commands.spawn((
        square_sprite.clone(),
        Transform::from_xyz(0.0, -50.0 * 6.0, 0.0).with_scale(Vec3::new(20.0, 1.0, 1.0))
    ));
    commands.spawn((
        square_sprite.clone(),
        Transform::from_xyz(-50.0 * 9.5, 0.0, 0.0).with_scale(Vec3::new(1.0, 11.0, 1.0))
    ));
    commands.spawn((
        square_sprite,
        Transform::from_xyz(50.0 * 9.5, 0.0, 0.0).with_scale(Vec3::new(1.0, 11.0, 1.0))
    ));

    let triangle = Triangle2d::new(
        Vec2::new(0.0, 10.0),
        Vec2::new(-10.0, -10.0),
        Vec2::new(10.0, -10.0),
    );

    let mut job = make_job();
    let entity = commands.spawn((
        Mesh2d(meshes.add(triangle)),
        Transform::from_xyz(0.0, 0.0, 1.0),
        Player
    )).id();
    job.assign(&mut commands, entity);

}
#[derive(Component, Reflect)]
struct Player;

fn make_job() -> Job {
    let mut tasks = JobTasks::new();

    tasks.first(Box::new(WaitTask{schedule: JobSchedule::RealDelay(2.0)}));
    tasks.next(Box::new(HideTask));
    tasks.next(Box::new(WaitTask{schedule: JobSchedule::RealDelay(2.0)}));
    tasks.next(Box::new(ShowTask));
    tasks.next(Box::new(WaitTask{schedule: JobSchedule::RealDelay(2.0)}));
    tasks.add_at(1000, Box::new(DespawnTask));

    return Job::new(
        JobData{
            name: "TestJob",
            tasks,
            on_fail: JobOnFail::Cancel
        }
    )
}
