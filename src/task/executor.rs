use super::{Task, TaskId};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;

const MAX_TASKS: usize = 100;

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

#[expect(clippy::new_without_default)]
impl Executor {
    #[must_use]
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(MAX_TASKS)),
            waker_cache: BTreeMap::new(),
        }
    }

    #[expect(
        clippy::missing_panics_doc,
        reason = "Panic is internal logic bug. Does not matter to caller"
    )]
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        assert!(
            self.tasks.insert(task.id, task).is_none(),
            "Task with same ID already exists in tasks"
        );
        self.task_queue.push(task_id).expect("queue full");
    }

    fn run_ready_tasks(&mut self) {
        while let Some(task_id) = self.task_queue.pop() {
            let Some(task) = self.tasks.get_mut(&task_id) else {
                continue; // Task no longer exists, skip it
            };

            let waker = self
                .waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, self.task_queue.clone()));

            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    // Taks is done. Remove it
                    self.tasks.remove(&task_id);
                    self.waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        interrupts::disable();
        if self.task_queue.is_empty() {
            // Wait for an interrupt
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    #[expect(clippy::new_ret_no_self)]
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
