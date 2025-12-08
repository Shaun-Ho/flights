use log::info;
pub type ThreadID = i32;
pub type TaskID = i32;

pub trait Runnable: Send + 'static {
    fn step(&mut self) -> bool;
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
    #[must_use]
    pub fn add_task<T>(&mut self, mut task: T, period: std::time::Duration) -> TaskID
    where
        T: Runnable,
    {
        let id = self.current_task_id;
        let running_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let thread_flag = running_flag.clone();
        let handle = std::thread::Builder::new()
            .name(std::any::type_name::<T>().to_string())
            .spawn(move || {
                while thread_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let time = std::time::Instant::now();
                    if !task.step() {
                        break;
                    }
                    let elapsed = time.elapsed();
                    if elapsed < period {
                        std::thread::sleep(period - elapsed);
                    }
                }
            })
            .expect("Failed to spawn thread");
        self.tasks.insert(
            id,
            ManagedTask {
                handle,
                running: running_flag,
            },
        );
        self.current_task_id += 1;
        id
    }

    pub fn stop_all_tasks(&self) {
        info!("Manager: Signaling all tasks to stop...");
        for task in self.tasks.values() {
            task.running
                .store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn wait_on_task_finish(&mut self, task_id: TaskID) {
        if let Some(task) = self.tasks.remove(&task_id) {
            let _ = task.handle.join();
        }
    }
}

impl Default for ThreadManager {
    fn default() -> Self {
        ThreadManager::new()
    }
}

struct ManagedTask {
    handle: std::thread::JoinHandle<()>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
#[cfg(test)]
mod tests {
    use super::{Runnable, ThreadManager};

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

    impl Runnable for CountingTask {
        fn step(&mut self) -> bool {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.sender.send(*count).unwrap();
            *count < self.limit
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

    impl Runnable for LoopingTask {
        fn step(&mut self) -> bool {
            let mut executions = self.executions.lock().unwrap();
            *executions += 1;
            self.sender.send(*executions).unwrap();
            true // Always return true, so it must be stopped externally
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
}
