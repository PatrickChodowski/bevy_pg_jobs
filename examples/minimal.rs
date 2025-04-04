use bevy::prelude::*;
use bevy::ecs::reflect::ReflectCommandExt;
use bevy_pg_jobs::*;
use bevy_pg_calendar::prelude::PGCalendarPlugin;

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
    .add_systems(Startup, init)
    .add_systems(Update, update)
    .run();
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct ID {
    id: usize
}
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct Player;

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
struct CosTam {
    name: String
}

fn init(
    mut commands: Commands
){

    let mut v: Vec<Box<&dyn Reflect>> = Vec::new();
    v.push(Box::new(ID{id: 0}.as_reflect()));
    v.push(Box::new(Player.as_reflect()));
    let b = CosTam{name: "dupa".to_string().clone()};
    v.push(Box::new(b.as_reflect()));
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