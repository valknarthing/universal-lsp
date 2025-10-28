// JavaScript Demo - Universal LSP Features
// This file demonstrates hover, completion, and diagnostics

class UserManager {
  constructor() {
    this.users = [];
  }

  addUser(name, email) {
    const user = {
      id: this.users.length + 1,
      name: name,
      email: email,
      createdAt: new Date()
    };
    this.users.push(user);
    return user;
  }

  findUserById(id) {
    return this.users.find(user => user.id === id);
  }

  getAllUsers() {
    return this.users;
  }
}

// Test the manager
const manager = new UserManager();
manager.addUser("Alice", "alice@example.com");
manager.addUser("Bob", "bob@example.com");

console.log(manager.getAllUsers());
