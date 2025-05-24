use bevy::prelude::*;
use bevy_pg_jobs::prelude::*;
use bevy_pg_calendar::prelude::PGCalendarPlugin;

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
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
){
    commands.spawn(Camera2d);

    let square_sprite = Sprite {
        color: Color::srgb(0.7, 0.7, 0.8),
        custom_size: Some(Vec2::splat(50.0)),
        ..default()
    };

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
        MeshMaterial2d(materials.add(Color::srgb(0.77, 0.87, 0.97))),
        Transform::from_xyz(0.0, 0.0, 1.0),
    )).id();
    job.assign(&mut commands, entity);

}


fn make_job() -> Job {
    let mut tasks = JobTasks::new();

    tasks.first(Box::new(WaitTask{schedule: JobSchedule::RealDelay(2.0)}), None);
    tasks.next(Box::new(HideTask), None);
    tasks.next(Box::new(WaitTask{schedule: JobSchedule::RealDelay(2.0)}), None);
    tasks.next(Box::new(ShowTask), None);
    tasks.next(Box::new(WaitTask{schedule: JobSchedule::RealDelay(2.0)}), Some(1000));
    tasks.add_at(1000, Box::new(DespawnTask), None);

    return Job::new(
        JobData{
            id: JobID(0),
            label: "TestJob".to_string(),
            fail_task_id: 1000,
            tasks
        }
    )
}
