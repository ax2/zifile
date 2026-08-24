use std::collections::VecDeque;
use std::fmt;

pub const DEFAULT_QUEUE_CAPACITY: usize = 32;

pub struct Job<T> {
    pub id: u64,
    pub payload: T,
}

impl<T> fmt::Debug for Job<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Job")
            .field("id", &self.id)
            .field("payload", &"<redacted>")
            .finish()
    }
}

pub enum Submission<T> {
    Start(Job<T>),
    Queued { id: u64, position: usize },
}

pub struct QueueFull<T> {
    pub payload: T,
    pub capacity: usize,
}

impl<T> fmt::Debug for QueueFull<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueueFull")
            .field("payload", &"<redacted>")
            .field("capacity", &self.capacity)
            .finish()
    }
}

pub struct OperationQueue<T> {
    active_id: Option<u64>,
    pending: VecDeque<Job<T>>,
    next_id: u64,
    capacity: usize,
}

impl<T> Default for OperationQueue<T> {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_QUEUE_CAPACITY)
    }
}

impl<T> fmt::Debug for OperationQueue<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationQueue")
            .field("active_id", &self.active_id)
            .field("pending_count", &self.pending.len())
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl<T> OperationQueue<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "operation queue capacity must be positive");
        Self {
            active_id: None,
            pending: VecDeque::new(),
            next_id: 1,
            capacity,
        }
    }

    pub fn submit(&mut self, payload: T) -> Result<Submission<T>, QueueFull<T>> {
        if self.len() >= self.capacity {
            return Err(QueueFull {
                payload,
                capacity: self.capacity,
            });
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let job = Job { id, payload };
        if self.active_id.is_none() {
            self.active_id = Some(id);
            Ok(Submission::Start(job))
        } else {
            self.pending.push_back(job);
            Ok(Submission::Queued {
                id,
                position: self.pending.len(),
            })
        }
    }

    pub fn complete(&mut self, id: u64) -> Result<Option<Job<T>>, CompletionError> {
        if self.active_id != Some(id) {
            return Err(CompletionError {
                expected: self.active_id,
                received: id,
            });
        }
        let next = self.pending.pop_front();
        self.active_id = next.as_ref().map(|job| job.id);
        Ok(next)
    }

    pub fn clear_pending(&mut self) -> Vec<T> {
        self.pending.drain(..).map(|job| job.payload).collect()
    }

    pub fn active_id(&self) -> Option<u64> {
        self.active_id
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn len(&self) -> usize {
        usize::from(self.active_id.is_some()) + self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.active_id.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionError {
    pub expected: Option<u64>,
    pub received: u64,
}

impl fmt::Display for CompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "operation completion mismatch: expected {:?}, received {}",
            self.expected, self.received
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn starts_first_job_and_preserves_fifo_order() {
        let mut queue = OperationQueue::with_capacity(3);
        let first = queue.submit("first").unwrap();
        let Submission::Start(first) = first else {
            panic!("first submission must start immediately");
        };
        assert_eq!(first.id, 1);
        assert_eq!(first.payload, "first");
        assert!(matches!(
            queue.submit("second"),
            Ok(Submission::Queued { id: 2, position: 1 })
        ));
        assert!(matches!(
            queue.submit("third"),
            Ok(Submission::Queued { id: 3, position: 2 })
        ));

        let second = queue.complete(1).unwrap().unwrap();
        assert_eq!((second.id, second.payload), (2, "second"));
        let third = queue.complete(2).unwrap().unwrap();
        assert_eq!((third.id, third.payload), (3, "third"));
        assert!(queue.complete(3).unwrap().is_none());
        assert!(queue.is_empty());
    }

    #[test]
    fn rejects_capacity_overflow_without_losing_payload() {
        let mut queue = OperationQueue::with_capacity(2);
        assert!(matches!(queue.submit(10), Ok(Submission::Start(_))));
        assert!(matches!(queue.submit(20), Ok(Submission::Queued { .. })));
        let error = queue.submit(30).err().expect("queue should be full");
        assert_eq!(error.payload, 30);
        assert_eq!(error.capacity, 2);
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn rejects_stale_completion_without_advancing_queue() {
        let mut queue = OperationQueue::default();
        let Submission::Start(first) = queue.submit("first").unwrap() else {
            panic!("first submission must start");
        };
        queue.submit("second").unwrap();
        let error = queue.complete(first.id + 10).unwrap_err();
        assert_eq!(error.expected, Some(first.id));
        assert_eq!(queue.active_id(), Some(first.id));
        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn clearing_pending_keeps_active_job() {
        let mut queue = OperationQueue::default();
        let Submission::Start(first) = queue.submit("first").unwrap() else {
            panic!("first submission must start");
        };
        queue.submit("second").unwrap();
        queue.submit("third").unwrap();
        assert_eq!(queue.clear_pending(), vec!["second", "third"]);
        assert_eq!(queue.active_id(), Some(first.id));
        assert_eq!(queue.pending_count(), 0);
        assert!(queue.complete(first.id).unwrap().is_none());
    }

    #[test]
    fn debug_output_redacts_payloads() {
        let secret = "not-for-debug-output";
        let job = Job {
            id: 7,
            payload: secret,
        };
        let full = QueueFull {
            payload: secret,
            capacity: 1,
        };
        assert!(!format!("{job:?}").contains(secret));
        assert!(!format!("{full:?}").contains(secret));
    }

    #[test]
    fn clearing_pending_drops_queued_payloads() {
        struct DropProbe(Rc<Cell<usize>>);
        impl Drop for DropProbe {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let drops = Rc::new(Cell::new(0));
        let mut queue = OperationQueue::default();
        let Submission::Start(active) = queue.submit(DropProbe(drops.clone())).unwrap() else {
            panic!("first submission must start");
        };
        queue.submit(DropProbe(drops.clone())).unwrap();
        drop(queue.clear_pending());
        assert_eq!(drops.get(), 1);
        drop(active);
        assert_eq!(drops.get(), 2);
    }
}
