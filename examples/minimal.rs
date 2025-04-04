use bevy::prelude::*;
use bevy::ecs::reflect::ReflectCommandExt;
use bevy_pg_jobs::prelude::*;
use bevy_pg_calendar::prelude::PGCalendarPlugin;

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
    .register_type::<ID>()
    .register_type::<Player>()
    .register_type::<CosTam>()
    .add_plugins(PGCalendarPlugin::default())
    // .add_plugins(PGJobsPlugin{
    //     active: true,
    //     debug: false
    // })
    .add_systems(Startup, setup)
    .add_systems(Startup, init)
    .add_systems(Update, update)
    .run();
}

fn setup(){

    let t = Task{
        id: 0, next: None, task: Box::new(Player)
    };

    let t1 = Task{
        id: 1, next: None, task: Box::new(ID{id: 0})
    };

}



#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct ID {
    id: usize
}
impl PGTask for ID {}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct Player;

impl PGTask for Player {}


#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
struct CosTam {
    name: String
}
impl PGTask for CosTam {}


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