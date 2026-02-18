use std::fs;

struct EnvConfig {
    access_token: Option<String>,
    v2_username: Option<String>,
    v2_password: Option<String>,
}

fn load_env() -> EnvConfig {
    let content = fs::read_to_string(".env").expect("Failed to read .env file");

    let mut client_id = None;
    let mut client_secret = None;
    let mut access_token = None;
    let mut v2_username = None;
    let mut v2_password = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key.trim() {
                "TICKTICK_CLIENT_ID" => client_id = Some(value.to_string()),
                "TICKTICK_CLIENT_SECRET" => client_secret = Some(value.to_string()),
                "TICKTICK_ACCESS_TOKEN" => access_token = Some(value.to_string()),
                "TICKTICK_USERNAME" => v2_username = Some(value.to_string()),
                "TICKTICK_PASSWORD" => v2_password = Some(value.to_string()),
                _ => {}
            }
        }
    }

    let _ = (
        client_id.expect("TICKTICK_CLIENT_ID not found"),
        client_secret.expect("TICKTICK_CLIENT_SECRET not found"),
    );

    EnvConfig {
        access_token,
        v2_username,
        v2_password,
    }
}

fn get_token() -> String {
    let env = load_env();
    env.access_token
        .expect("TICKTICK_ACCESS_TOKEN not found in .env - required for API tests")
}

fn load_v2_credentials() -> (String, String) {
    let env = load_env();
    let username = env
        .v2_username
        .expect("TICKTICK_USERNAME not found in .env - required for v2 API tests");
    let password = env
        .v2_password
        .expect("TICKTICK_PASSWORD not found in .env - required for v2 API tests");
    (username, password)
}

const BASE_URL: &str = "https://api.ticktick.com/open/v1";

#[test]
#[ignore]
fn test_get_all_projects() {
    let token = get_token();

    let resp = ureq::get(&format!("{BASE_URL}/project"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_array(), "Response should be an array");

    println!("Projects: {}", serde_json::to_string_pretty(&body).unwrap());
}

#[test]
#[ignore]
fn test_get_project_by_id() {
    let token = get_token();

    // First get all projects to find an ID
    let resp = ureq::get(&format!("{BASE_URL}/project"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    let projects: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");

    if projects.is_empty() {
        println!("No projects found, skipping test");
        return;
    }

    let project_id = projects[0]["id"].as_str().expect("Project should have id");
    println!("Testing with project ID: {project_id}");

    let resp = ureq::get(&format!("{BASE_URL}/project/{project_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_object(), "Response should be an object");
    assert!(body["id"].is_string(), "Project should have id");
    assert!(body["name"].is_string(), "Project should have name");

    println!("Project: {}", serde_json::to_string_pretty(&body).unwrap());
}

#[test]
#[ignore]
fn test_get_project_data() {
    let token = get_token();

    // First get all projects to find an ID
    let resp = ureq::get(&format!("{BASE_URL}/project"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    let projects: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");

    if projects.is_empty() {
        println!("No projects found, skipping test");
        return;
    }

    let project_id = projects[0]["id"].as_str().expect("Project should have id");
    println!("Testing with project ID: {project_id}");

    let resp = ureq::get(&format!("{BASE_URL}/project/{project_id}/data"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_object(), "Response should be an object");

    println!(
        "Project data: {}",
        serde_json::to_string_pretty(&body).unwrap()
    );
}

#[test]
#[ignore]
fn test_create_and_delete_task() {
    let token = get_token();

    // First get all projects to find an ID
    let resp = ureq::get(&format!("{BASE_URL}/project"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    let projects: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");

    if projects.is_empty() {
        println!("No projects found, skipping test");
        return;
    }

    let project_id = projects[0]["id"].as_str().expect("Project should have id");
    println!("Testing with project ID: {project_id}");

    // Create a task
    let task_body = serde_json::json!({
        "title": "Test task from CLI",
        "projectId": project_id
    });

    let resp = ureq::post(&format!("{BASE_URL}/task"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(task_body)
        .expect("Create task failed");

    assert_eq!(resp.status(), 200);

    let created_task: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(created_task["id"].is_string(), "Task should have id");

    let task_id = created_task["id"].as_str().unwrap();
    println!(
        "Created task: {}",
        serde_json::to_string_pretty(&created_task).unwrap()
    );

    // Delete the task
    let resp = ureq::delete(&format!("{BASE_URL}/project/{project_id}/task/{task_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("Delete task failed");

    assert_eq!(resp.status(), 200);
    println!("Task deleted successfully");
}

#[test]
#[ignore]
fn test_complete_task() {
    let token = get_token();

    // First get all projects to find an ID
    let resp = ureq::get(&format!("{BASE_URL}/project"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    let projects: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");

    if projects.is_empty() {
        println!("No projects found, skipping test");
        return;
    }

    let project_id = projects[0]["id"].as_str().expect("Project should have id");
    println!("Testing with project ID: {project_id}");

    // Create a task to complete
    let task_body = serde_json::json!({
        "title": "Test task to complete",
        "projectId": project_id
    });

    let resp = ureq::post(&format!("{BASE_URL}/task"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(task_body)
        .expect("Create task failed");

    let created_task: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let task_id = created_task["id"].as_str().unwrap();
    println!("Created task: {task_id}");

    // Complete the task
    let resp = ureq::post(&format!(
        "{BASE_URL}/project/{project_id}/task/{task_id}/complete"
    ))
    .set("Authorization", &format!("Bearer {token}"))
    .call()
    .expect("Complete task failed");

    assert_eq!(resp.status(), 200);
    println!("Task completed successfully");

    // Clean up - delete the task
    let _ = ureq::delete(&format!("{BASE_URL}/project/{project_id}/task/{task_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    println!("Task deleted (cleanup)");
}

#[test]
#[ignore]
fn test_move_task_between_projects() {
    let token = get_token();

    // Get at least two projects
    let resp = ureq::get(&format!("{BASE_URL}/project"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    let projects: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");

    if projects.len() < 2 {
        println!("Need at least 2 projects to test move, skipping");
        return;
    }

    let source_id = projects[0]["id"].as_str().expect("Project should have id");
    let dest_id = projects[1]["id"].as_str().expect("Project should have id");
    println!("Moving task from project {source_id} to {dest_id}");

    // Create a task in the source project
    let task_body = serde_json::json!({
        "title": "Test move task",
        "projectId": source_id
    });

    let resp = ureq::post(&format!("{BASE_URL}/task"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(task_body)
        .expect("Create task failed");

    let created_task: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let task_id = created_task["id"].as_str().unwrap();
    println!("Created task: {task_id}");

    // Move the task by updating its projectId
    let move_body = serde_json::json!({
        "taskId": task_id,
        "projectId": dest_id,
    });

    let resp = ureq::post(&format!("{BASE_URL}/task/{task_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(move_body)
        .expect("Move task failed");

    assert_eq!(resp.status(), 200);
    println!("Move POST returned status 200");

    // Verify the task appears in the destination project data
    let resp = ureq::get(&format!("{BASE_URL}/project/{dest_id}/data"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("Get project data failed");

    let data: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let tasks = data["tasks"].as_array().expect("tasks should be an array");
    let found = tasks.iter().any(|t| t["id"].as_str() == Some(task_id));
    assert!(
        found,
        "Moved task should appear in destination project data"
    );
    println!("Task found in destination project");

    // Also verify the individual task GET endpoint behavior (may return empty body)
    let individual_resp = ureq::get(&format!("{BASE_URL}/project/{dest_id}/task/{task_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match individual_resp {
        Ok(resp) => {
            let body = resp.into_string().unwrap_or_default();
            if body.is_empty() {
                println!("Individual GET returned empty body (confirming API behavior)");
            } else {
                println!("Individual GET returned: {body}");
            }
        }
        Err(e) => println!("Individual GET failed: {e} (confirming API behavior)"),
    }

    // Clean up
    let _ = ureq::delete(&format!("{BASE_URL}/project/{dest_id}/task/{task_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();
    println!("Task deleted (cleanup)");
}

const V2_BASE_URL: &str = "https://api.ticktick.com/api/v2";

#[test]
#[ignore]
fn test_v2_move_task_between_projects() {
    let token = get_token();
    let (username, password) = load_v2_credentials();

    // Get at least two projects
    let resp = ureq::get(&format!("{BASE_URL}/project"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    let projects: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");

    if projects.len() < 2 {
        println!("Need at least 2 projects to test v2 move, skipping");
        return;
    }

    let source_id = projects[0]["id"].as_str().expect("Project should have id");
    let dest_id = projects[1]["id"].as_str().expect("Project should have id");
    println!("v2 move: project {source_id} -> {dest_id}");

    // Create a task in the source project via v1 API
    let task_body = serde_json::json!({
        "title": "Test v2 move task",
        "projectId": source_id
    });

    let resp = ureq::post(&format!("{BASE_URL}/task"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(task_body)
        .expect("Create task failed");

    let created_task: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let task_id = created_task["id"].as_str().unwrap();
    println!("Created task: {task_id}");

    // Sign in to v2 API
    let device_id = "6490test00000000000000";
    let x_device = format!(
        r#"{{"platform":"web","os":"macOS 10.15.7","device":"Chrome 130.0.0.0","name":"","version":6490,"id":"{device_id}","channel":"website","campaign":"","websocket":""}}"#
    );

    let signon_body = serde_json::json!({
        "username": username,
        "password": password,
    });

    let resp = ureq::post(&format!("{V2_BASE_URL}/user/signon?wc=true&remember=true"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Content-Type", "application/json")
        .send_json(signon_body)
        .expect("v2 signon failed");

    let signon_resp: serde_json::Value = resp.into_json().expect("Failed to parse signon JSON");
    let session_token = signon_resp["token"]
        .as_str()
        .expect("Missing token in signon response");
    println!("v2 session authenticated");

    // Move via v2 batch endpoint
    let move_body = serde_json::json!([{
        "taskId": task_id,
        "fromProjectId": source_id,
        "toProjectId": dest_id,
    }]);

    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/taskProject"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(move_body)
        .expect("v2 move failed");

    assert_eq!(resp.status(), 200);
    println!("v2 move returned status 200");

    // Verify task appears in destination via v1 project data endpoint
    let resp = ureq::get(&format!("{BASE_URL}/project/{dest_id}/data"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("Get project data failed");

    let data: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let tasks = data["tasks"].as_array().expect("tasks should be an array");
    let found = tasks.iter().any(|t| t["id"].as_str() == Some(task_id));
    assert!(
        found,
        "v2-moved task should appear in destination project data"
    );
    println!("Task found in destination project after v2 move");

    // Verify same task ID was preserved (not recreated)
    let moved_task = tasks
        .iter()
        .find(|t| t["id"].as_str() == Some(task_id))
        .unwrap();
    assert_eq!(moved_task["title"].as_str(), Some("Test v2 move task"));
    println!("Task ID and title preserved after v2 move");

    // Clean up
    let _ = ureq::delete(&format!("{BASE_URL}/project/{dest_id}/task/{task_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();
    println!("Task deleted (cleanup)");
}
