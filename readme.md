# Bevy PG Jobs

A template code for Bevy game engine that I use in my games.

!! Its not a ready plugin !!

In order to use it you need to add it to your game and edit the pg_tasks file with your own tasks, structs, and systems

## How it works

0) Define and add job to `Res<Jobs>` resource. Internally it uses bevy_pg_calendar to check if the job can be triggered
1) It triggers job on 
    - Cron schedule
    - RealDelay (real time seconds)
    - Delay (calendar delay in hours)
    - Instant (starts immediately)
    - OnDemand  (its in resource but it can be triggered by another system only)
2) job.start will create new empty entity with only one component - component from the first task (I assume it is SpawnTask)
3) You should define the system that would handle SpawnTask. After the task its done the SpawnTask should be removed from the entity using commands. Then call jobs.next_task to insert new component for the entity to start the new task.
4) Example tasks implemented (Variants of TaskType enum)
    - Spawn
    - Despawn
    - Hide
    - Show
    - Teleport
    - Move
    - Rotate
    - Wait
    - Decision Task 
    - Loop NK Task (last n steps for k times)
    - Loop N Task (last n steps till broken manually)

5) Decision tasks will make a decision and switch to some other task
6) Loop tasks will loop over last n tasks


## Todo
- wait for event task
- send event task
- insert component task
- decision task
- loop n task
- loop nk task
- text debug layer
- read job from text file
- JobCatalog example with predefined task
- dynamic task 
- allow prejobs if needed (maybe not needed anymore?)
- is task status needed now?

