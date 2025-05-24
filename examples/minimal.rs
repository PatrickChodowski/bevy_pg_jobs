use bevy::prelude::*;
use bevy::ecs::reflect::ReflectCommandExt;
use bevy_pg_jobs::prelude::*;
use pg_jobs_macros::PGTask;
use bevy_pg_calendar::prelude::PGCalendarPlugin;
use serde::{Serialize, Deserialize};

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
    .register_type::<ID>()
    .register_type::<Player>()
    .register_type::<CosTam>()
    .add_plugins(PGCalendarPlugin::default())
    .add_plugins(PGJobsPlugin{
        active: true,
        debug: false
    })
    .add_systems(Startup, setup)
    .add_systems(Startup, init)
    .add_systems(Update, update)
    .run();
}

fn setup(){
    let mut tasks = JobTasks::new();
    tasks.first(Box::new(Player), None);
    tasks.next(Box::new(ID{id: 0}), None);
}



#[derive(Component, Reflect, Debug, Clone, Serialize, Deserialize, PGTask)]
#[reflect(Component)]
struct ID {
    id: usize
}

#[derive(Component, Reflect, Debug, Clone, Serialize, Deserialize, PGTask)]
#[reflect(Component)]
struct Player;

#[derive(Component, Reflect, Debug, Clone, Serialize, Deserialize, PGTask)]
#[reflect(Component)]
struct CosTam {
    name: String
}


fn init(
    mut commands: Commands
){

    let mut v: Vec<Box<&dyn PGTask>> = Vec::new();
    v.push(Box::new(&ID{id: 0}));
    v.push(Box::new(&Player));
    let b = CosTam{name: "dupa22".to_string().clone()};
    v.push(Box::new(&b));
    let entity_id = commands.spawn_empty().id();

    for partial_comp in v.iter(){
        let reflected = partial_comp.reflect_clone().ok().unwrap().into_partial_reflect();
        commands.entity(entity_id).insert_reflect(reflected);
    }

}


fn update(
    query: Query<(&ID, &Player, &CosTam)>
){
    for (id, player, costam) in query.iter(){
        info!("{:?}, {:?}, {:?}", id, player, costam);
    }

}