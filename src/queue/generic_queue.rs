use std::collections::VecDeque;
use std::sync::Mutex;
use std::marker::PhantomData;
use std::fmt::Debug;

pub struct GenericQueue<T: Clone + Debug> {
    queue: Mutex<VecDeque<T>>,
    _marker: PhantomData<T>,
}

impl<T: Clone + Debug> GenericQueue<T> {
    pub fn new() -> Self {
        GenericQueue {
            queue: Mutex::new(VecDeque::new()),
            _marker: PhantomData,
        }
    }
    
    pub fn enqueue(&self, message: T) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(message);
    }
    
    pub fn dequeue(&self) -> Option<T> {
        let mut queue = self.queue.lock().unwrap();
        queue.pop_front()
    }
    
    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }
    
    pub fn size(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
    
    pub fn peek(&self) -> Option<T> {
        let queue = self.queue.lock().unwrap();
        queue.front().cloned()
    }
}