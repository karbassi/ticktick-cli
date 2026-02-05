use std::fs;

fn load_env() -> (String, String, Option<String>) {
    let content = fs::read_to_string(".env").expect("Failed to read .env file");

    let mut client_id = None;
    let mut client_secret = None;
    let mut access_token = None;

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
                _ => {}
            }
        }
    }

    (
        client_id.expect("TICKTICK_CLIENT_ID not found"),
        client_secret.expect("TICKTICK_CLIENT_SECRET not found"),
        access_token,
    )
}

fn get_token() -> String {
    let (_, _, token) = load_env();
    token.expect("TICKTICK_ACCESS_TOKEN not found in .env - required for API tests")
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
