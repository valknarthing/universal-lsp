# Python Demo - Universal LSP Features
# This file demonstrates hover, completion, and diagnostics

from datetime import datetime
from typing import List, Dict, Optional

class Task:
    def __init__(self, title: str, description: str):
        self.id = id(self)
        self.title = title
        self.description = description
        self.completed = False
        self.created_at = datetime.now()

    def mark_complete(self) -> None:
        """Mark this task as completed"""
        self.completed = True

    def to_dict(self) -> Dict[str, any]:
        """Convert task to dictionary"""
        return {
            "id": self.id,
            "title": self.title,
            "description": self.description,
            "completed": self.completed,
            "created_at": self.created_at.isoformat()
        }

class TaskManager:
    def __init__(self):
        self.tasks: List[Task] = []

    def add_task(self, title: str, description: str) -> Task:
        """Add a new task to the manager"""
        task = Task(title, description)
        self.tasks.append(task)
        return task

    def get_task(self, task_id: int) -> Optional[Task]:
        """Get a task by ID"""
        for task in self.tasks:
            if task.id == task_id:
                return task
        return None

    def list_tasks(self, completed: Optional[bool] = None) -> List[Task]:
        """List all tasks, optionally filtered by completion status"""
        if completed is None:
            return self.tasks
        return [t for t in self.tasks if t.completed == completed]

# Demo usage
if __name__ == "__main__":
    manager = TaskManager()
    task1 = manager.add_task("Write docs", "Document the Universal LSP")
    task2 = manager.add_task("Add tests", "Write unit tests")

    task1.mark_complete()

    print("All tasks:", len(manager.list_tasks()))
    print("Completed:", len(manager.list_tasks(completed=True)))
    print("Pending:", len(manager.list_tasks(completed=False)))
