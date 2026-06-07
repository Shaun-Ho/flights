use log;

pub type ThreadID = i32;
pub type TaskID = i32;

#[derive(Debug, PartialEq, Eq)]
pub enum TaskState {
    // Task is still running
    Running,
    // Task defines itself as completed
    Completed,
    // Task encountered an error, and had to terminate
    Errored(String),
}

pub trait SteppableTask: Send + 'static {
    fn step(&mut self) -> TaskState;
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
    /// - `&mut self` (`undefined`) - Describe this parameter.
    /// - `mut task` (`T`) - Task to be added - this type must implement the `Runnable` trait.
    ///   This repeatedly tries to run the task at the specified `period` interval
    /// - `period` (`std`) - Period between tasks running.
    ///
    /// # Returns
    ///
    /// - `TaskID where T: Runnable,` - Task. ID.
    ///
    /// # Panics
    ///
    /// Will panic if thread does not spawn successfully.
    pub fn add_task<T>(&mut self, task: T, period: std::time::Duration) -> TaskID
    where
        T: SteppableTask,
    {
        let id = self.current_task_id;

        let task_status = std::sync::Arc::new(std::sync::RwLock::new(ThreadStatus::Active));
        let worker_status = task_status.clone();
        let (stop_sender, stop_receiver) = crossbeam_channel::bounded::<()>(1);

        let thread_task: Box<dyn FnOnce() + Send> = if period.is_zero() {
            Box::new(move || {
                run_task_continuously(task, &stop_receiver, worker_status);
            })
        } else {
            Box::new(move || {
                run_task_with_period(task, period, &stop_receiver, worker_status);
            })
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
    Active,          // Task has been submitted to an active thread
    Interrupted,     // interrupted by a receiving a stop command
    Completed,       // Task completed successfully - mapped from `TaskState::Completed`
    Errored(String), // Task encountered an error - mapped from `TaskState::Errored`
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

fn run_task_with_period<T: SteppableTask>(
    mut task: T,
    period: std::time::Duration,
    stop_receiver: &crossbeam_channel::Receiver<()>,
    task_status: std::sync::Arc<std::sync::RwLock<ThreadStatus>>,
) {
    let mut next_iteration_time = std::time::Instant::now();
    loop {
        // Check if we are interrupted
        match stop_receiver.try_recv() {
            Ok(()) | Err(crossbeam_channel::TryRecvError::Disconnected) => {
                *task_status.write().unwrap() = ThreadStatus::Interrupted;
                break;
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
        }

        // Find what is the time for next iteration
        next_iteration_time += period;

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

        // Check if sleep is needed if task completed within the window
        let now = std::time::Instant::now();

        if next_iteration_time < now {
            let sleep_duration = next_iteration_time - now;
            std::thread::sleep(sleep_duration);
        } else {
            next_iteration_time = now;
        }
    }
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
    use crate::core::thread_manager::TaskState;

    use super::{SteppableTask, ThreadManager};

    // A simple runnable task for counting and self-stopping
    #[derive(Debug)]
    struct CountingTask {
        count: std::sync::Arc<std::sync::Mutex<usize>>,
        limit: usize,
        sender: std::sync::mpsc::Sender<usize>,
    }

    impl CountingTask {
        fn new(limit: usize, sender: std::sync::mpsc::Sender<usize>) -> Self {
            Self {
                count: std::sync::Arc::new(std::sync::Mutex::new(0)),
                limit,
                sender,
            }
        }
    }

    impl SteppableTask for CountingTask {
        fn step(&mut self) -> TaskState {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.sender.send(*count).unwrap();
            if *count < self.limit {
                TaskState::Running
            } else {
                TaskState::Completed
            }
        }
    }

    // A runnable task that runs indefinitely until stopped externally
    #[derive(Debug)]
    struct LoopingTask {
        sender: std::sync::mpsc::Sender<usize>,
        executions: std::sync::Arc<std::sync::Mutex<usize>>,
    }

    impl LoopingTask {
        fn new(sender: std::sync::mpsc::Sender<usize>) -> Self {
            Self {
                sender,
                executions: std::sync::Arc::new(std::sync::Mutex::new(0)),
            }
        }
    }

    impl SteppableTask for LoopingTask {
        fn step(&mut self) -> TaskState {
            let mut executions = self.executions.lock().unwrap();
            *executions += 1;
            self.sender.send(*executions).unwrap();
            TaskState::Running
        }
    }

    #[test]
    fn when_multiple_tasks_added_then_all_tasks_completed() {
        let mut manager = ThreadManager::new();
        let (counter_1_sender, counter_1_receiver) = std::sync::mpsc::channel();
        let (counter_2_sender, counter_2_receiver) = std::sync::mpsc::channel();

        let counter_1_limit = 5;
        let counter_2_limit = 10;
        let task_1 = CountingTask::new(counter_1_limit, counter_1_sender);
        let task_2 = CountingTask::new(counter_2_limit, counter_2_sender);
        let task_1_id = manager.add_task(task_1, std::time::Duration::from_millis(50));
        let task_2_id = manager.add_task(task_2, std::time::Duration::from_millis(50));

        manager.wait_on_task_finish(task_2_id);
        manager.wait_on_task_finish(task_1_id);

        // check that all tasks have been run
        assert!(manager.tasks.is_empty());

        // check that all tasks executed
        let counter_2_messages: Vec<usize> = counter_2_receiver.try_iter().collect();
        let counter_1_messages: Vec<usize> = counter_1_receiver.try_iter().collect();
        assert_eq!(counter_1_messages.len(), counter_1_limit);
        assert_eq!(counter_2_messages.len(), counter_2_limit);
    }

    #[test]
    fn when_stop_all_tasks_is_called() {
        let mut manager = ThreadManager::new();
        let (counter_sender, counter_receiver) = std::sync::mpsc::channel();
        let (looper_sender, _) = std::sync::mpsc::channel();

        let counter_limit = 5;
        let counter_task = CountingTask::new(counter_limit, counter_sender);
        let looping_task = LoopingTask::new(looper_sender);
        let counter_task_id = manager.add_task(counter_task, std::time::Duration::from_millis(50));
        let looping_task_id = manager.add_task(looping_task, std::time::Duration::from_millis(50));

        // give ample time for counter to be executed
        std::thread::sleep(std::time::Duration::from_millis(counter_limit as u64 * 100));
        manager.stop_all_tasks();
        manager.wait_on_task_finish(counter_task_id);
        manager.wait_on_task_finish(looping_task_id);

        assert!(manager.tasks.is_empty());

        let counter_messages: Vec<usize> = counter_receiver.try_iter().collect();
        assert_eq!(counter_messages.len(), counter_limit);
    }

    #[test]
    fn when_wait_on_task_finish_called_then_task_id_removed() {
        let mut manager = ThreadManager::new();
        let (sender, _receiver) = std::sync::mpsc::channel();

        let task_id1 = manager.add_task(
            LoopingTask::new(sender.clone()),
            std::time::Duration::from_millis(100),
        );
        let task_id2 = manager.add_task(
            LoopingTask::new(sender.clone()),
            std::time::Duration::from_millis(100),
        );

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

        let task_1_id = manager.add_task(
            LoopingTask::new(looper_1_sender),
            std::time::Duration::from_millis(10),
        );
        let task_2_id = manager.add_task(
            LoopingTask::new(looper_2_sender),
            std::time::Duration::from_millis(10),
        );

        // Give them a moment to start executing
        std::thread::sleep(std::time::Duration::from_millis(50));

        let stop_result = manager.stop_task(task_1_id);
        assert!(stop_result.is_ok(), "Stopping existing task should succeed");
        manager.wait_on_task_finish(task_1_id);

        let executions_task_1: Vec<usize> = looper_1_receiver.try_iter().collect();

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
        let executions_task2: Vec<usize> = looper_2_receiver.try_iter().collect();

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

        let _ = manager.add_task(
            LoopingTask::new(looper_1_sender),
            std::time::Duration::from_millis(10),
        );
        let _ = manager.add_task(
            LoopingTask::new(looper_2_sender),
            std::time::Duration::from_millis(10),
        );

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
