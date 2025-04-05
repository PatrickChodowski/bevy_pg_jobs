
How to insert from reflect:

```

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

```