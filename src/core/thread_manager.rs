use log;

pub type ThreadID = i32;
pub type TaskID = i32;

#[derive(Debug)]
pub enum TaskState {
    // Task is still running
    Running,
    // Task defines itself as completed
    Completed,
    // Task encountered an error, and had to terminate
    Errored(Box<dyn std::error::Error + Send + Sync>),
}

pub trait SteppableTask: Send + 'static {
    fn step(&mut self) -> TaskState;
}

pub enum TaskSchedule {
    Continuous,
    Periodic(PeriodicTask),
}

pub struct PeriodicTask {
    period: std::time::Duration,
    overrun_policy: OverrunPolicy,
}
impl PeriodicTask {
    pub fn new(
        period: std::time::Duration,
        overrun_policy: OverrunPolicy,
    ) -> Result<Self, ZeroPeriodError> {
        if period.is_zero() {
            return Err(ZeroPeriodError);
        }
        Ok(Self {
            period,
            overrun_policy,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Periodic task period must be non-zero")]
pub struct ZeroPeriodError;
pub enum OverrunPolicy {
    Drop,
}

impl<T: SteppableTask + ?Sized> SteppableTask for Box<T> {
    fn step(&mut self) -> TaskState {
        (**self).step()
    }
}
pub struct ThreadManager {
    current_task_id: ThreadID,
    tasks: std::collections::HashMap<ThreadID, ManagedTask>,
}

impl ThreadManager {
    #[must_use]
    pub fn new() -> Self {
        ThreadManager {
            current_task_id: 0,
            tasks: std::collections::HashMap::new(),
        }
    }

    #[must_use]
    pub fn current_task_id(&self) -> TaskID {
        self.current_task_id
    }

    /// Adds a task to the thread manager.
    ///
    /// # Arguments
    ///
    /// - `task` (`T`) - Task to be added - this type must implement the `SteppableTask` trait.
    /// - `schedule` (`TaskSchedule`) - How often the task is stepped: back-to-back
    ///   (`Continuous`) or once per `period` (`Periodic`), where the `overrun_policy`
    ///   decides what happens to ticks missed because a step ran late.
    ///
    /// # Returns
    ///
    /// - `TaskID where T: Runnable,` - Task. ID.
    ///
    /// # Panics
    ///
    /// Will panic if thread does not spawn successfully.
    pub fn add_task<T>(&mut self, task: T, task_schedule: TaskSchedule) -> TaskID
    where
        T: SteppableTask,
    {
        let id = self.current_task_id;

        let task_status = std::sync::Arc::new(std::sync::RwLock::new(ThreadStatus::Active));
        let worker_status = task_status.clone();
        let (stop_sender, stop_receiver) = crossbeam_channel::bounded::<()>(1);

        let thread_task: Box<dyn FnOnce() + Send> = match task_schedule {
            TaskSchedule::Continuous => Box::new(move || {
                run_task_continuously(task, &stop_receiver, worker_status);
            }),
            TaskSchedule::Periodic(periodic_task) => Box::new(move || {
                run_task_periodically(task, periodic_task, &stop_receiver, worker_status);
            }),
        };

        let handle = std::thread::Builder::new()
            .name(std::any::type_name::<T>().to_string())
            .spawn(move || {
                thread_task();
            })
            .expect("Failed to spawn thread");
        self.tasks.insert(
            id,
            ManagedTask {
                task_id: id,
                handle,
                stop_sender,
                status: task_status,
            },
        );
        self.current_task_id += 1;
        id
    }

    pub fn stop_task(&self, task_id: TaskID) -> Result<(), crossbeam_channel::SendError<()>> {
        let task = self
            .tasks
            .get(&task_id)
            .ok_or(crossbeam_channel::SendError(()))?;

        task.stop_sender.send(())
    }
    pub fn stop_all_tasks(&self) {
        log::info!("ThreadManager: Signaling all tasks to stop...");
        for task in self.tasks.values() {
            let _ = task.stop_sender.send(());
        }
    }

    pub fn wait_on_task_finish(&mut self, task_id: TaskID) {
        if let Some(task) = self.tasks.remove(&task_id) {
            log_task_finished_status(task);
        }
    }

    pub fn wait_on_all_tasks(&mut self) {
        if self.tasks.is_empty() {
            return;
        }
        log::info!("Waiting for all {} tasks to finish", self.tasks.len());

        for (_id, task) in self.tasks.drain() {
            log_task_finished_status(task);
        }
    }
}

impl Default for ThreadManager {
    fn default() -> Self {
        ThreadManager::new()
    }
}

impl Drop for ThreadManager {
    fn drop(&mut self) {
        // If the developer used your API correctly (wait_on_all_tasks),
        // this is empty and Drop does absolutely nothing.
        if self.tasks.is_empty() {
            return;
        }

        let remaining = self.tasks.len();
        log::warn!(
            "ThreadManager dropping with {remaining} tasks remaining. Enforcing bounded wait..."
        );

        let timeout_duration = std::time::Duration::from_secs(5);
        let start_time = std::time::Instant::now();

        let mut remaining_tasks: Vec<_> = self
            .tasks
            .drain()
            .map(|(id, task)| (id, task.handle))
            .collect();

        // Loop until tasks finish OR we hit the timeout
        while !remaining_tasks.is_empty() && start_time.elapsed() < timeout_duration {
            let mut i = 0;
            while i < remaining_tasks.len() {
                if remaining_tasks[i].1.is_finished() {
                    let (id, handle) = remaining_tasks.remove(i);
                    match handle.join() {
                        Ok(_) => log::debug!("Task {} completed during drop period.", id),
                        Err(e) => log::error!("Task {id} panicked during drop: {e:?}"),
                    }
                } else {
                    i += 1;
                }
            }

            if !remaining_tasks.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }

        if !remaining_tasks.is_empty() {
            log::error!(
                "Drop complete: {} tasks did not finish in time and have been detached.",
                remaining_tasks.len()
            );
        }
    }
}

#[derive(Debug)]
enum ThreadStatus {
    Active,      // Task has been submitted to an active thread
    Interrupted, // interrupted by a receiving a stop command
    Completed,   // Task completed successfully - mapped from `TaskState::Completed`
    Errored(Box<dyn std::error::Error + Send + Sync>), // Task encountered an error - mapped from `TaskState::Errored`
}

struct ManagedTask {
    task_id: TaskID,
    handle: std::thread::JoinHandle<()>,
    stop_sender: crossbeam_channel::Sender<()>,
    status: std::sync::Arc<std::sync::RwLock<ThreadStatus>>,
}

fn run_task_continuously<T: SteppableTask>(
    mut task: T,
    stop_receiver: &crossbeam_channel::Receiver<()>,
    task_status: std::sync::Arc<std::sync::RwLock<ThreadStatus>>,
) {
    loop {
        // Check if we are interrupted
        match stop_receiver.try_recv() {
            Ok(()) | Err(crossbeam_channel::TryRecvError::Disconnected) => {
                *task_status.write().unwrap() = ThreadStatus::Interrupted;
                break;
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
        }
        // Check if we should still loop on the task
        match task.step() {
            TaskState::Running => {} // do nothing, task continuing
            TaskState::Completed => {
                *task_status.write().unwrap() = ThreadStatus::Completed;
                break;
            }
            TaskState::Errored(error) => {
                *task_status.write().unwrap() = ThreadStatus::Errored(error);
                break;
            }
        }

        std::thread::yield_now();
    }
}

fn run_task_periodically<T: SteppableTask>(
    mut task: T,
    periodic_task: PeriodicTask,
    stop_receiver: &crossbeam_channel::Receiver<()>,
    task_status: std::sync::Arc<std::sync::RwLock<ThreadStatus>>,
) {
    let PeriodicTask {
        period,
        overrun_policy,
    } = periodic_task;

    let mut next_iteration_time = std::time::Instant::now();
    loop {
        // Run task & update the task status
        match task.step() {
            TaskState::Running => {} // do nothing here
            TaskState::Completed => {
                *task_status.write().unwrap() = ThreadStatus::Completed;
                break;
            }
            TaskState::Errored(error) => {
                *task_status.write().unwrap() = ThreadStatus::Errored(error);
                break;
            }
        }

        // Find what is the time for next iteration
        next_iteration_time += period;
        let now = std::time::Instant::now();
        if next_iteration_time <= now {
            match overrun_policy {
                OverrunPolicy::Drop => {
                    next_iteration_time = next_deadline_after(next_iteration_time, period, now);
                }
            }
        }

        match stop_receiver.recv_deadline(next_iteration_time) {
            Ok(()) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                *task_status.write().unwrap() = ThreadStatus::Interrupted;
                break;
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
        }
    }
}

fn next_deadline_after(
    missed_deadline: std::time::Instant,
    period: std::time::Duration,
    now: std::time::Instant,
) -> std::time::Instant {
    let late_by = now.duration_since(missed_deadline);
    let periods_missed = late_by.as_nanos() / period.as_nanos() + 1;
    let periods_missed = u32::try_from(periods_missed).unwrap_or(u32::MAX);
    missed_deadline + period * periods_missed
}

fn log_task_finished_status(task: ManagedTask) {
    let ManagedTask {
        task_id,
        handle,
        status,
        ..
    } = task;
    match handle.join() {
        Ok(_) => {
            let final_status = status.read().unwrap();
            match &*final_status {
                ThreadStatus::Active => {
                    log::warn!("Task {task_id} exited abnormally without updating its status.")
                }
                ThreadStatus::Interrupted => {
                    log::info!("Task {task_id}: was interrupted.")
                }
                ThreadStatus::Completed => {
                    log::info!("Task {task_id} completed successfully")
                }
                ThreadStatus::Errored(err) => {
                    log::error!("Task was interrupted due to error: {err}")
                }
            }
        }
        Err(e) => {
            log::error!("Task {task_id} panicked: {e:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A simple runnable task for counting and self-stopping.
    // Sends the time each step started, so tests can assert on scheduling.
    #[derive(Debug)]
    struct CountingTask {
        count: std::sync::Arc<std::sync::Mutex<usize>>,
        limit: usize,
        sender: std::sync::mpsc::Sender<std::time::Instant>,
        step_duration: std::time::Duration,
        delayed_step: Option<(usize, std::time::Duration)>,
    }

    impl CountingTask {
        fn new(limit: usize, sender: std::sync::mpsc::Sender<std::time::Instant>) -> Self {
            Self {
                count: std::sync::Arc::new(std::sync::Mutex::new(0)),
                limit,
                sender,
                step_duration: std::time::Duration::ZERO,
                delayed_step: None,
            }
        }

        fn with_step_duration(mut self, step_duration: std::time::Duration) -> Self {
            self.step_duration = step_duration;
            self
        }

        fn with_delayed_step(mut self, step: usize, delay: std::time::Duration) -> Self {
            self.delayed_step = Some((step, delay));
            self
        }
    }

    impl SteppableTask for CountingTask {
        fn step(&mut self) -> TaskState {
            self.sender.send(std::time::Instant::now()).unwrap();
            std::thread::sleep(self.step_duration);
            let mut count = self.count.lock().unwrap();
            if let Some((step, delay)) = self.delayed_step
                && step == *count
            {
                std::thread::sleep(delay);
            }
            *count += 1;
            if *count < self.limit {
                TaskState::Running
            } else {
                TaskState::Completed
            }
        }
    }

    // A runnable task that runs indefinitely until stopped externally.
    // Sends the time each step started, so tests can assert on scheduling.
    #[derive(Debug)]
    struct LoopingTask {
        sender: std::sync::mpsc::Sender<std::time::Instant>,
    }

    impl LoopingTask {
        fn new(sender: std::sync::mpsc::Sender<std::time::Instant>) -> Self {
            Self { sender }
        }
    }

    impl SteppableTask for LoopingTask {
        fn step(&mut self) -> TaskState {
            self.sender.send(std::time::Instant::now()).unwrap();
            TaskState::Running
        }
    }
    mod threadmanager_setup_and_shutdown_ordering {
        use super::*;

        #[test]
        fn when_multiple_tasks_self_ending_tasks_added_then_graceful_shutdown_when_all_tasks_completed()
         {
            let mut manager = ThreadManager::new();
            let (counter_1_sender, counter_1_receiver) = std::sync::mpsc::channel();
            let (counter_2_sender, counter_2_receiver) = std::sync::mpsc::channel();

            let counter_1_limit = 5;
            let counter_2_limit = 10;
            let task_1 = CountingTask::new(counter_1_limit, counter_1_sender);
            let task_2 = CountingTask::new(counter_2_limit, counter_2_sender);
            let task_1_id = manager.add_task(task_1, TaskSchedule::Continuous);
            let task_2_id = manager.add_task(task_2, TaskSchedule::Continuous);

            manager.wait_on_task_finish(task_2_id);
            manager.wait_on_task_finish(task_1_id);

            // check that all tasks have been run
            assert!(manager.tasks.is_empty());

            // check that all tasks executed
            let counter_2_messages: Vec<std::time::Instant> =
                counter_2_receiver.try_iter().collect();
            let counter_1_messages: Vec<std::time::Instant> =
                counter_1_receiver.try_iter().collect();
            assert_eq!(counter_1_messages.len(), counter_1_limit);
            assert_eq!(counter_2_messages.len(), counter_2_limit);
        }

        #[test]
        fn given_never_ending_task_is_running_when_stop_all_tasks_called_then_tasks_are_shutdown_gracefully()
         {
            let mut manager = ThreadManager::new();
            let (counter_sender, counter_receiver) = std::sync::mpsc::channel();
            let (looper_sender, _) = std::sync::mpsc::channel();

            let counter_limit = 5;
            let counter_task = CountingTask::new(counter_limit, counter_sender);
            let looping_task = LoopingTask::new(looper_sender);
            let counter_task_id = manager.add_task(counter_task, TaskSchedule::Continuous);
            let looping_task_id = manager.add_task(looping_task, TaskSchedule::Continuous);

            // give ample time for counter to be executed
            std::thread::sleep(std::time::Duration::from_millis(counter_limit as u64 * 100));
            manager.stop_all_tasks();
            manager.wait_on_task_finish(counter_task_id);
            manager.wait_on_task_finish(looping_task_id);

            assert!(manager.tasks.is_empty());

            let counter_messages: Vec<std::time::Instant> = counter_receiver.try_iter().collect();
            assert_eq!(counter_messages.len(), counter_limit);
        }

        #[test]
        fn when_wait_on_task_finish_called_then_task_id_removed() {
            let mut manager = ThreadManager::new();
            let (sender, _receiver) = std::sync::mpsc::channel();

            let task_id1 =
                manager.add_task(LoopingTask::new(sender.clone()), TaskSchedule::Continuous);
            let task_id2 =
                manager.add_task(LoopingTask::new(sender.clone()), TaskSchedule::Continuous);

            assert_eq!(manager.tasks.len(), 2);

            manager.stop_all_tasks();
            manager.wait_on_task_finish(task_id1);

            assert_eq!(manager.tasks.len(), 1);
            assert!(manager.tasks.contains_key(&task_id2));
            assert!(!manager.tasks.contains_key(&task_id1));

            manager.wait_on_task_finish(task_id2);
            assert!(manager.tasks.is_empty()); // No tasks left
        }

        #[test]
        fn when_specific_task_is_stopped_then_task_is_removed_from_threadmanager() {
            let mut manager = ThreadManager::new();
            let (looper_1_sender, looper_1_receiver) = std::sync::mpsc::channel();
            let (looper_2_sender, looper_2_receiver) = std::sync::mpsc::channel();

            let task_1_id =
                manager.add_task(LoopingTask::new(looper_1_sender), TaskSchedule::Continuous);
            let task_2_id =
                manager.add_task(LoopingTask::new(looper_2_sender), TaskSchedule::Continuous);

            // Give them a moment to start executing
            std::thread::sleep(std::time::Duration::from_millis(50));

            let stop_result = manager.stop_task(task_1_id);
            assert!(stop_result.is_ok(), "Stopping existing task should succeed");
            manager.wait_on_task_finish(task_1_id);

            let executions_task_1: Vec<std::time::Instant> = looper_1_receiver.try_iter().collect();

            assert!(
                !executions_task_1.is_empty(),
                "Task 1 should have executed at least once after asking to stop"
            );
            // check that task no longer exists
            assert!(!manager.tasks.contains_key(&task_1_id));

            // Verify the other task is still running (or can be stopped)
            println!("Verifying task {task_2_id} is still running (or stoppable)");
            std::thread::sleep(std::time::Duration::from_millis(50));

            let stop_result_2 = manager.stop_task(task_2_id);
            assert!(stop_result_2.is_ok(), "Stopping task 2 should succeed");

            manager.wait_on_task_finish(task_2_id);
            let executions_task2: Vec<std::time::Instant> = looper_2_receiver.try_iter().collect();

            assert!(
                !executions_task2.is_empty(),
                "Task 2 should have executed at least once after asking to stop"
            );
            assert!(!manager.tasks.contains_key(&task_2_id));

            assert!(manager.tasks.is_empty());
        }

        #[test]
        fn when_non_existent_task_is_stopped_then_task_is_removed_from_threadmanager() {
            let mut manager = ThreadManager::new();
            let (looper_1_sender, _looper_1_receiver) = std::sync::mpsc::channel();
            let (looper_2_sender, _looper_2_receiver) = std::sync::mpsc::channel();

            let _ = manager.add_task(LoopingTask::new(looper_1_sender), TaskSchedule::Continuous);
            let _ = manager.add_task(LoopingTask::new(looper_2_sender), TaskSchedule::Continuous);

            std::thread::sleep(std::time::Duration::from_millis(50));

            let non_existent_task_id = 999;
            let stop_non_existent_result = manager.stop_task(non_existent_task_id);
            assert!(
                stop_non_existent_result.is_err(),
                "Stopping a non-existent task should return an error"
            );
            assert_eq!(
                stop_non_existent_result.unwrap_err(),
                crossbeam_channel::SendError(()),
                "Error for non-existent task should be SendError(())"
            );
        }
    }
    mod given_periodic_task_timings {
        use super::*;

        mod with_drop_policy {
            use super::*;

            #[test]
            fn when_task_finishes_within_period_then_each_step_starts_on_or_after_its_tick() {
                let mut manager = ThreadManager::new();
                let (sender, receiver) = std::sync::mpsc::channel();
                let period = std::time::Duration::from_millis(50);
                let limit = 4;

                let start = std::time::Instant::now();
                let task_id = manager.add_task(
                    CountingTask::new(limit, sender),
                    TaskSchedule::Periodic(PeriodicTask::new(period, OverrunPolicy::Drop).unwrap()),
                );
                manager.wait_on_task_finish(task_id);

                let step_times: Vec<std::time::Instant> = receiver.try_iter().collect();
                assert_eq!(step_times.len(), limit);
                for (k, step_time) in (0u32..).zip(&step_times) {
                    assert!(
                        *step_time >= start + period * k,
                        "step {k} started {:?} after start, before its tick at {:?}",
                        step_time.duration_since(start),
                        period * k
                    );
                }
            }

            #[test]
            fn given_task_overruns_period_when_drop_policy_then_missed_ticks_are_skipped() {
                let mut manager = ThreadManager::new();
                let (sender, receiver) = std::sync::mpsc::channel();
                let period = std::time::Duration::from_millis(40);
                let limit = 4;

                // Each step takes 1.5 periods, so it always misses the next tick. Dropping that
                // tick puts step `k` on tick `2k`; catching up would instead run steps
                // back-to-back, 1.5 periods apart
                let start = std::time::Instant::now();

                let task_id = manager.add_task(
                    CountingTask::new(limit, sender).with_step_duration(period * 3 / 2),
                    TaskSchedule::Periodic(PeriodicTask::new(period, OverrunPolicy::Drop).unwrap()),
                );
                manager.wait_on_task_finish(task_id);

                let step_times: Vec<std::time::Instant> = receiver.try_iter().collect();
                assert_eq!(step_times.len(), limit);

                for (k, step_time) in (0u32..).zip(&step_times) {
                    assert!(
                        *step_time >= start + period * (2 * k),
                        "step {k} started {:?} after start, before its tick at {:?}",
                        step_time.duration_since(start),
                        period * (2 * k)
                    );
                }
            }

            #[test]
            fn given_one_step_overruns_period_then_missed_ticks_are_skipped_and_period_resumes() {
                let mut manager = ThreadManager::new();
                let (sender, receiver) = std::sync::mpsc::channel();
                let period = std::time::Duration::from_millis(40);

                let expected_ticks = [0u32, 1, 4, 5];
                let start = std::time::Instant::now();
                let task_id = manager.add_task(
                    CountingTask::new(expected_ticks.len(), sender)
                        .with_delayed_step(1, period * 5 / 2),
                    TaskSchedule::Periodic(PeriodicTask::new(period, OverrunPolicy::Drop).unwrap()),
                );
                manager.wait_on_task_finish(task_id);

                let step_times: Vec<std::time::Instant> = receiver.try_iter().collect();
                assert_eq!(step_times.len(), expected_ticks.len());
                for (k, (step_time, tick)) in step_times.iter().zip(expected_ticks).enumerate() {
                    assert!(
                        *step_time >= start + period * tick,
                        "step {k} started {:?} after start, before its tick at {:?}",
                        step_time.duration_since(start),
                        period * tick
                    );
                }

                // Loose upper bound (a whole period of slack): the last step must not have
                // skipped past its tick, i.e. the normal period resumed after the overrun
                let last_tick = expected_ticks[expected_ticks.len() - 1];
                let last_step_offset = step_times[step_times.len() - 1].duration_since(start);
                assert!(
                    last_step_offset < period * (last_tick + 1),
                    "last step started {last_step_offset:?} after start, a whole period past its tick at {:?}",
                    period * last_tick
                );
            }

            #[test]
            fn given_long_period_task_when_stopped_then_shuts_down_without_waiting_for_period() {
                let mut manager = ThreadManager::new();
                let (sender, _receiver) = std::sync::mpsc::channel();

                let task_id = manager.add_task(
                    LoopingTask::new(sender),
                    TaskSchedule::Periodic(
                        PeriodicTask::new(std::time::Duration::from_secs(60), OverrunPolicy::Drop)
                            .unwrap(),
                    ),
                );
                std::thread::sleep(std::time::Duration::from_millis(50));

                let start = std::time::Instant::now();
                manager.stop_task(task_id).unwrap();
                manager.wait_on_task_finish(task_id);
                assert!(start.elapsed() < std::time::Duration::from_secs(1));
            }

            #[test]
            fn when_deadlines_missed_then_drop_policy_skips_to_next_future_tick() {
                let base = std::time::Instant::now();
                let period = std::time::Duration::from_millis(10);

                // 25ms late: ticks at +10 and +20 are dropped, next is +30
                let next =
                    next_deadline_after(base, period, base + std::time::Duration::from_millis(25));
                assert_eq!(next, base + std::time::Duration::from_millis(30));

                // Exactly on a tick boundary: that tick has passed, so skip to the following one
                let next = next_deadline_after(base, period, base + period);
                assert_eq!(next, base + period * 2);
            }
        }
    }
}
