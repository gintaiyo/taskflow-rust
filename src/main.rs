
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::{get, post},
    Form,
    Router,
};

use askama::Template;
use serde::{Deserialize, Serialize};

use std::{
    cmp::Reverse,
    io,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::{
    fs,
    sync::RwLock,
};

use tower_http::services::ServeDir;
use uuid::Uuid;


// priority btn
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Priority {
    Low,
    Normal,
    High,
}

impl Priority {
    fn css_class(&self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Normal => "normal",
            Priority::High => "high",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Normal => "Normal",
            Priority::High => "High",
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Priority::Low => 1,
            Priority::Normal => 2,
            Priority::High => 3,
        }
    }
}


// Task structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: Uuid,
    title: String,
    priority: Priority,
    done: bool,
}


// Application
// Arc: Allows multiple parts of our application to share ownership.
// RwLock: Allows multiple readers OR one writer.
// Vec<Task>: Our in-memory task collection.

#[derive(Clone)]
struct AppState {
    tasks: Arc<RwLock<Vec<Task>>>,
}


// New task aka form data
#[derive(Deserialize)]
struct NewTask {
    title: String,
    priority: Priority,
}


// Sort things
#[derive(Deserialize, Default)]
struct SortQuery {
    sort: Option<String>,
    notice: Option<String>,
}


// Askama
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    tasks: Vec<Task>,
    total_count: usize,
    pending_count: usize,
    completed_count: usize,
    current_sort: String,
    flash_message: String,
}


// Quotes shown after actions.
// Sorting does not use a notice, so it won't show a quote.

fn get_quote(notice: Option<&str>) -> String {
    let quotes: &[&str] = match notice {
        Some("added") => &[
            "One less thing to worry about. Well, eventually.",
            "Another task enters the battlefield.",
            "Your future self says thanks.",
            "Added. Now comes the hard part.",
            "Small steps, big productivity.",
            "Your to-do list just got a little longer.",
            "Look at you, making plans.",
            "Another one for the list. Good luck.",
        ],

        Some("completed") => &[
            "Look at you, getting things done.",
            "Another one bites the dust.",
            "Progress is progress. Keep going.",
            "You actually did it. Impressive.",
            "That's one less thing on your mind.",
            "Task defeated. What's next?",
            "Small victory. Take it.",
            "Productivity looks good on you.",
        ],

        Some("undone") => &[
            "Back on the battlefield, soldier.",
            "Not finished yet? No worries.",
            "Everyone deserves a second chance.",
            "This task isn't escaping that easily.",
            "The grind continues.",
            "A little setback never hurt anyone.",
        ],

        Some("deleted") => &[
            "Gone. Just like your motivation.",
            "Sometimes letting go is productive.",
            "Out of sight, out of mind.",
            "One less thing taking up space.",
            "Well, that task had a good run.",
            "Making room for better things.",
        ],

        Some("cleared") => &[
            "A clean slate. Don't mess it up.",
            "The past is gone. Start fresh.",
            "Nothing left behind. Nice.",
            "Fresh start. New problems.",
            "Your list is finally breathing again.",
            "Peace at last. For now.",
        ],

        Some("blank") => &[
            "You need to give me something to work with.",
            "A task without a title? Interesting.",
            "Even I can't read your mind.",
            "Come on, give that task a name.",
            "Empty titles aren't going to get things done.",
        ],

        _ => return String::new(),
    };

    // Pick a quote using the current time.
    let index = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize
        % quotes.len();

    quotes[index].to_string()
}


// JSON

const DATABASE_FILE: &str = "db.json";

async fn load_tasks() -> io::Result<Vec<Task>> {
    match fs::read(DATABASE_FILE).await {

        // File exists: convert JSON into Vec<Task>.
        Ok(contents) => {
            let tasks = serde_json::from_slice(&contents)
                .map_err(io::Error::other)?;

            Ok(tasks)
        }

        // File doesn't exist yet.
        Err(error) if error.kind() == io::ErrorKind::NotFound => {

            // Create an empty JSON array.
            fs::write(DATABASE_FILE, "[]").await?;

            Ok(Vec::new())
        }

        // Some other file-reading error.
        Err(error) => Err(error),
    }
}


// Saving in JSON

async fn save_tasks(tasks: &[Task]) -> io::Result<()> {
    let json = serde_json::to_vec_pretty(tasks)
        .map_err(io::Error::other)?;

    fs::write(DATABASE_FILE, json).await?;

    Ok(())
}


// Main page
// /?sort=priority = High to Low
// /?sort=status = Incomplete to Complete

async fn index(
    State(state): State<AppState>,
    Query(params): Query<SortQuery>,
) -> Result<Html<String>, (StatusCode, String)> {

    // Read and clone the tasks for sorting and display.
    let mut tasks = state.tasks.read().await.clone();

    // Which sorting option was selected?
    let current_sort = match params.sort.as_deref() {
        Some("priority") => "priority",
        Some("status") => "status",
        _ => "default",
    };

    if current_sort == "priority" {
        tasks.sort_by_key(|task| {
            Reverse(task.priority.rank())
        });
    }
    else if current_sort == "status" {
        tasks.sort_by_key(|task| task.done);
    }

    // Dashboard statistics.
    let total_count = tasks.len();

    let completed_count = tasks
        .iter()
        .filter(|task| task.done)
        .count();

    let pending_count = total_count - completed_count;

    // Generate a quote if an action provided a notice.
    let flash_message = get_quote(params.notice.as_deref());

    // Give our data to Askama.
    let template = IndexTemplate {
        tasks,
        total_count,
        pending_count,
        completed_count,
        current_sort: current_sort.to_string(),
        flash_message,
    };

    // Convert the template into HTML.
    let html = template
        .render()
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;

    Ok(Html(html))
}


// Add task
// Get HTML form and convert it into a Task.

async fn add_task(
    State(state): State<AppState>,
    Form(form): Form<NewTask>,
) -> Result<Redirect, (StatusCode, String)> {

    let title = form.title.trim();

    if title.is_empty() {
        return Ok(Redirect::to("/?notice=blank"));
    }

    let task = Task {
        id: Uuid::new_v4(),
        title: title.to_string(),
        priority: form.priority,
        done: false,
    };

    let mut tasks = state.tasks.write().await;

    tasks.push(task);

    // Save the updated collection to JSON.
    save_tasks(&tasks)
        .await
        .map_err(internal_error)?;

    Ok(Redirect::to("/?notice=added"))
}


// Completed / incomplete toggle

async fn toggle_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Redirect, (StatusCode, String)> {

    let mut tasks = state.tasks.write().await;

    // Find the task with the matching UUID.
    let notice = match tasks.iter_mut().find(|task| task.id == id) {

        Some(task) => {
            task.done = !task.done;

            if task.done {
                "completed"
            } else {
                "undone"
            }
        }

        None => {
            return Err((
                StatusCode::NOT_FOUND,
                "Task not found".to_string(),
            ));
        }
    };

    save_tasks(&tasks)
        .await
        .map_err(internal_error)?;

    Ok(Redirect::to(&format!("/?notice={notice}")))
}


// Delete task one by one.

async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Redirect, (StatusCode, String)> {

    let mut tasks = state.tasks.write().await;

    let old_length = tasks.len();

    tasks.retain(|task| task.id != id);

    if tasks.len() == old_length {
        return Err((
            StatusCode::NOT_FOUND,
            "Task not found".to_string(),
        ));
    }

    // Save the updated collection.
    save_tasks(&tasks)
        .await
        .map_err(internal_error)?;

    Ok(Redirect::to("/?notice=deleted"))
}


// Remove everything. I mean tasks gng.

async fn clear_tasks(
    State(state): State<AppState>,
) -> Result<Redirect, (StatusCode, String)> {

    let mut tasks = state.tasks.write().await;

    // Remove everything.
    tasks.clear();

    // Save the empty collection to JSON.
    save_tasks(&tasks)
        .await
        .map_err(internal_error)?;

    Ok(Redirect::to("/?notice=cleared"))
}


// Errors

fn internal_error(
    error: io::Error,
) -> (StatusCode, String) {

    eprintln!("Storage error: {error}");

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Something went wrong while saving your tasks."
            .to_string(),
    )
}


// Main function

#[tokio::main]
async fn main() {

    let saved_tasks = load_tasks()
        .await
        .expect("Could not load db.json");

    println!("Loaded {} saved tasks.", saved_tasks.len());

    let state = AppState {
        tasks: Arc::new(RwLock::new(saved_tasks)),
    };

    let app = Router::new()

        .route("/", get(index))

        .route("/", post(add_task))

        .route("/tasks/{id}/toggle", post(toggle_task))

        .route("/tasks/{id}/delete", post(delete_task))

        .route("/tasks/clear", post(clear_tasks))

        .nest_service(
            "/static",
            ServeDir::new("static"),
        )

        .with_state(state);

    let listener = tokio::net::TcpListener::bind(
        "0.0.0.0:3000"
    )
    .await
    .expect("Failed to bind address");

    println!("Task Manager running!");
    println!("http://localhost:3000");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}
