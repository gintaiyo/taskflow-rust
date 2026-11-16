# TaskFlow

A simple task manager built using Rust, Axum, Askama, HTML and CSS.

I wanted to build something with Rust that was a little more practical than just writing CLI programs. So I decided to make a task manager where I could learn how Axum works, how to handle routes, manage shared state and work with HTML templates.

The UI uses a glassmorphism-inspired dark theme with a simple dashboard layout.

## Features

- Add tasks with a custom title.
- Set task priority (Low, Normal, High).
- Mark tasks as completed or undo completion.
- Delete individual tasks.
- Clear all tasks at once.
- Sort tasks by priority (High to Low).
- Sort tasks by status (Incomplete first).
- Dashboard showing total, pending and completed tasks.
- Save tasks in a local JSON file so they don't disappear after restarting the server.
- Responsive UI for different screen sizes.

No JavaScript is used. All interactions are handled through HTML forms and links.

## Tech Stack

- Rust
- Axum
- Tokio
- Askama
- Serde
- Serde JSON
- UUID
- Tower HTTP
- HTML5
- CSS3

## Project Structure

```text
taskify/
├── Cargo.toml
├── Cargo.lock
├── db.json
├── src/
│   └── main.rs
├── templates/
│   └── index.html
├── static/
│   ├── style.css
│   └── room-bg.svg
└── README.md
```

## Getting Started

### Clone the repository

```bash
git clone <your-repository-url>
cd taskify
```

### Run the project

```bash
cargo run
```

### Open in browser

```text
http://127.0.0.1:3000
```

## Storage

Tasks are stored in `db.json`.

The application uses `Arc<RwLock<Vec<Task>>>` to manage shared task data.

Tasks are loaded from the JSON file when the server starts and saved whenever something changes.

This means restarting the server won't delete your tasks.

## Routes

| Method | Route | Description |
|---|---|---|
| GET | `/` | Dashboard |
| POST | `/` | Add a task |
| POST | `/tasks/{id}/toggle` | Toggle task status |
| POST | `/tasks/{id}/delete` | Delete a task |
| POST | `/tasks/clear` | Clear all tasks |
| GET | `/?sort=priority` | Sort by priority |
| GET | `/?sort=status` | Sort by status |

## What I Learned

This project helped me understand:

- How Axum handles HTTP requests and routes.
- How shared application state works using `Arc` and `RwLock`.
- How Askama renders HTML templates.
- How to handle HTML form submissions without JavaScript.
- How Serde converts Rust structs into JSON and back.
- How to perform asynchronous file operations with Tokio.
- How to structure a full-stack Rust application.

## Note

This project currently uses local JSON file storage instead of an actual database like PostgreSQL or MongoDB.

It's mainly a learning project, but I wanted to make something that actually works and looks decent rather than just another tutorial example.
