# Bevy PG Jobs

This wont work as plugin, more like a template because the tasks need to be adjusted per game


### Ideas:
- system id and one shot system per task would have work but those systems are run sequentially and it simply might not be performant enough for large amount of entities

- instead, maybe the task manager should insert a component for each task? But its very close to the tasktype

- Better task type then whole HashMap of tasks on each component -> that can end up as A LOT of memory usage

- using trait in a query would cause dynamic dispatch on every run so not sure about that either. Probably want to avoid it for so many entities.



### Step by step:

1) Define WorkerTask as PGTask and Component. This will be passed to the entity
2) write a system that handles it.
3) Dispatcher will assign the WorkerTasks to entities
4) Task systems will manage the task


### Questions:
Where to store the entity jobs? CompileJobCatalog not working, there are dynamic jobs with parameters

### TODO
- solve PGTrait derive_macro enigma
- add option for SparseSet and Table storages
- Add Schedule for Task Evaluation and Task systems (within update)
- Figure out if and how to implement Clone for the Job
