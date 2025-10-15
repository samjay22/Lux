//! Async executor implementation
//!
//! This module implements the async task executor for Lux.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use crate::runtime::value::{Value, FunctionValue};
use crate::parser::ast::Stmt;

/// Task ID for tracking async tasks
pub type TaskId = usize;

/// Task state
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Running,
    Completed(Value),
    Failed(String),
}

/// Async task with function and arguments
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub body: Vec<Stmt>,
    pub state: TaskState,
    pub function: Option<FunctionValue>,
    pub arguments: Vec<Value>,
}

impl Task {
    pub fn new(id: TaskId, name: String, body: Vec<Stmt>) -> Self {
        Self {
            id,
            name,
            body,
            state: TaskState::Pending,
            function: None,
            arguments: Vec::new(),
        }
    }

    pub fn new_with_function(id: TaskId, function: FunctionValue, arguments: Vec<Value>) -> Self {
        Self {
            id,
            name: function.name.clone(),
            body: function.body.clone(),
            state: TaskState::Pending,
            function: Some(function),
            arguments,
        }
    }
}

/// Async executor with goroutine-style task spawning
pub struct AsyncExecutor {
    tasks: Arc<Mutex<Vec<Task>>>,
    ready_queue: Arc<Mutex<VecDeque<TaskId>>>,
    next_task_id: Arc<Mutex<TaskId>>,
}

impl AsyncExecutor {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            ready_queue: Arc::new(Mutex::new(VecDeque::new())),
            next_task_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Spawn a new async task
    pub fn spawn(&self, name: String, body: Vec<Stmt>) -> TaskId {
        let mut next_id = self.next_task_id.lock().unwrap();
        let task_id = *next_id;
        *next_id += 1;

        let task = Task::new(task_id, name, body);

        let mut tasks = self.tasks.lock().unwrap();
        tasks.push(task);

        let mut queue = self.ready_queue.lock().unwrap();
        queue.push_back(task_id);

        task_id
    }

    /// Spawn a new async task with function and arguments
    pub fn spawn_function(&self, function: FunctionValue, arguments: Vec<Value>) -> TaskId {
        let mut next_id = self.next_task_id.lock().unwrap();
        let task_id = *next_id;
        *next_id += 1;

        let task = Task::new_with_function(task_id, function, arguments);

        let mut tasks = self.tasks.lock().unwrap();
        tasks.push(task);

        // Don't add to ready queue - tasks will be executed when awaited

        task_id
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: TaskId) -> Option<Task> {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().find(|t| t.id == task_id).cloned()
    }

    /// Update task state
    pub fn update_task_state(&self, task_id: TaskId, state: TaskState) {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.state = state;
        }
    }

    /// Get next ready task
    pub fn get_next_ready_task(&self) -> Option<TaskId> {
        let mut queue = self.ready_queue.lock().unwrap();
        queue.pop_front()
    }

    /// Check if all tasks are complete
    pub fn all_tasks_complete(&self) -> bool {
        let tasks = self.tasks.lock().unwrap();
        let queue = self.ready_queue.lock().unwrap();

        queue.is_empty() && tasks.iter().all(|t| {
            matches!(t.state, TaskState::Completed(_) | TaskState::Failed(_))
        })
    }

    /// Get all completed tasks
    pub fn get_completed_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter()
            .filter(|t| matches!(t.state, TaskState::Completed(_)))
            .cloned()
            .collect()
    }

    /// Get all failed tasks
    pub fn get_failed_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter()
            .filter(|t| matches!(t.state, TaskState::Failed(_)))
            .cloned()
            .collect()
    }

    /// Clear all tasks
    pub fn clear(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        let mut queue = self.ready_queue.lock().unwrap();
        tasks.clear();
        queue.clear();
    }
}

