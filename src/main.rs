
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
};

use tokio::{
    fs,
    sync::RwLock,
};

use tower_http::services::ServeDir;
use uuid::Uuid;


//priority btn
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


//Task structure

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: Uuid,
    title: String,
    priority: Priority,
    done: bool,
}


//Application
//I forget below smart pointers so to know where to use il wrote this
// Arc:Allows multiple parts of our application to share ownership of the same data.
// RwLock:Allows multiple readers OR one writer.
// Vec<Task>:Our in-memory task collection.

#[derive(Clone)]
struct AppState {
    tasks: Arc<RwLock<Vec<Task>>>,
}


//new task aka form data

#[derive(Deserialize)]
struct NewTask {
    title: String,
    priority: Priority,
}

// sort things

#[derive(Deserialize, Default)]
struct SortQuery {
    sort: Option<String>,
}


//askama i find its name funny 

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    tasks: Vec<Task>,
    total_count: usize,
    pending_count: usize,
    completed_count: usize,
    current_sort: String,
}


// jSON

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


// saving in jSON

async fn save_tasks(tasks: &[Task]) -> io::Result<()> {

    let json = serde_json::to_vec_pretty(tasks)
        .map_err(io::Error::other)?;

    fs::write(DATABASE_FILE, json).await?;

    Ok(())
}


// main page
// it displays dashboard and does following things
// /?sort=priority = High to Low
// /?sort=status = Incomplete to Complete

async fn index(
    State(state): State<AppState>,
    Query(params): Query<SortQuery>,
) -> Result<Html<String>, (StatusCode, String)> {

    // Read, clone in jSON for sorting and display it

    let mut tasks = state.tasks.read().await.clone();

    // which option was selected man rust is goated with this feature
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

    let total_count = tasks.len();

    let completed_count = tasks
        .iter()
        .filter(|task| task.done)
        .count();

    let pending_count = total_count - completed_count;

    // Give our data to Askama.
    let template = IndexTemplate {
        tasks,
        total_count,
        pending_count,
        completed_count,
        current_sort: current_sort.to_string(),
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


// add task 
// get html form convert to jSON

async fn add_task(
    State(state): State<AppState>,
    Form(form): Form<NewTask>,
) -> Result<Redirect, (StatusCode, String)> {

    let title = form.title.trim();

    if title.is_empty() {
        return Ok(Redirect::to("/"));
    }

    let task = Task {
        id: Uuid::new_v4(),
        title: title.to_string(),
        priority: form.priority,
        done: false,
    };

    let mut tasks = state.tasks.write().await;

    tasks.push(task);

    // Save the updated collection to jSON

    save_tasks(&tasks)
        .await
        .map_err(internal_error)?;


    Ok(Redirect::to("/"))
}


// completed/incomplete toggle 

async fn toggle_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Redirect, (StatusCode, String)> {

    let mut tasks = state.tasks.write().await;

    // Find the task with the matching UUID.
    let task = tasks
        .iter_mut()
        .find(|task| task.id == id);

    match task {

        Some(task) => {

            task.done = !task.done;

            save_tasks(&tasks)
                .await
                .map_err(internal_error)?;

            Ok(Redirect::to("/"))
        }

        None => Err((
            StatusCode::NOT_FOUND,
            "Task not found".to_string(),
        )),
    }
}


//delete task one by one slow and steady

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

    Ok(Redirect::to("/"))
}


//remove everything i meant task gng

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

    Ok(Redirect::to("/"))
}


//errors not real errors handling them

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


//main function so it runs

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

    println!("🚀 Task Manager running!");
    println!("🌐 http://localhost:3000");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}
