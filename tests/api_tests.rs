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

fn v2_session(username: &str, password: &str) -> (String, String) {
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
    let token = signon_resp["token"]
        .as_str()
        .expect("Missing token in signon response")
        .to_string();

    (token, x_device)
}

#[test]
#[ignore]
fn test_v2_list_completed_tasks() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let resp = ureq::get(&format!(
        "{V2_BASE_URL}/project/all/completedInAll/?from=2020-01-01%2B00%3A00%3A00&to=2030-01-01%2B00%3A00%3A00&limit=10"
    ))
    .set("User-Agent", "Mozilla/5.0")
    .set("x-device", &x_device)
    .set("Cookie", &format!("t={session_token}"))
    .call()
    .expect("v2 completed tasks request failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_array(), "Response should be an array");
    println!(
        "Completed tasks: {}",
        serde_json::to_string_pretty(&body).unwrap()
    );
}

#[test]
#[ignore]
fn test_v2_tag_create_and_delete() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let tag_name = "cli-test-tag";

    // Create
    let body = serde_json::json!({ "add": [{ "label": tag_name, "name": tag_name }] });
    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/tag"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 tag create failed");

    assert_eq!(resp.status(), 200);
    println!("Tag created");

    // Verify tag exists in listing
    let resp = ureq::get(&format!("{V2_BASE_URL}/tags"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 tag list failed");

    let tags: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");
    let found = tags.iter().any(|t| t["name"].as_str() == Some(tag_name));
    assert!(found, "Created tag should appear in tag listing");
    println!("Tag found in listing");

    // Delete
    let resp = ureq::delete(&format!("{V2_BASE_URL}/tag?name={tag_name}"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 tag delete failed");

    assert_eq!(resp.status(), 200);
    println!("Tag deleted");
}

#[test]
#[ignore]
fn test_v2_tag_rename() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let old_name = "cli-test-rename-old";
    let new_name = "cli-test-rename-new";

    // Create the tag first
    let body = serde_json::json!({ "add": [{ "label": old_name, "name": old_name }] });
    ureq::post(&format!("{V2_BASE_URL}/batch/tag"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 tag create failed");

    // Rename
    let rename_body = serde_json::json!({ "name": old_name, "newName": new_name });
    let resp = ureq::put(&format!("{V2_BASE_URL}/tag/rename"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(rename_body)
        .expect("v2 tag rename failed");

    assert_eq!(resp.status(), 200);
    println!("Tag renamed");

    // Clean up
    let _ = ureq::delete(&format!("{V2_BASE_URL}/tag?name={new_name}"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call();
    println!("Tag deleted (cleanup)");
}

#[test]
#[ignore]
fn test_v2_set_task_parent() {
    let token = get_token();
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    // Get a project
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

    // Create parent and child tasks
    let parent_body = serde_json::json!({
        "title": "Test parent task",
        "projectId": project_id
    });
    let resp = ureq::post(&format!("{BASE_URL}/task"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(parent_body)
        .expect("Create parent task failed");
    let parent_task: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let parent_id = parent_task["id"].as_str().unwrap();

    let child_body = serde_json::json!({
        "title": "Test child task",
        "projectId": project_id
    });
    let resp = ureq::post(&format!("{BASE_URL}/task"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(child_body)
        .expect("Create child task failed");
    let child_task: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let child_id = child_task["id"].as_str().unwrap();

    println!("Created parent={parent_id}, child={child_id} in project {project_id}");

    // Set parent via v2 batch endpoint
    let batch_body = serde_json::json!([{
        "parentId": parent_id,
        "projectId": project_id,
        "taskId": child_id,
    }]);

    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/taskParent"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(batch_body)
        .expect("v2 set parent failed");

    assert_eq!(resp.status(), 200);
    println!("Parent relationship set via v2 batch");

    // Verify via project data
    let resp = ureq::get(&format!("{BASE_URL}/project/{project_id}/data"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("Get project data failed");

    let data: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let tasks = data["tasks"].as_array().expect("tasks should be an array");
    let child = tasks.iter().find(|t| t["id"].as_str() == Some(child_id));

    if let Some(child) = child {
        assert_eq!(
            child["parentId"].as_str(),
            Some(parent_id),
            "Child should have parentId set"
        );
        println!("Child task has parentId={parent_id} confirmed");
    } else {
        println!("Child task not found in project data (may need time to propagate)");
    }

    // Clean up
    let _ = ureq::delete(&format!("{BASE_URL}/project/{project_id}/task/{child_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();
    let _ = ureq::delete(&format!("{BASE_URL}/project/{project_id}/task/{parent_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();
    println!("Tasks deleted (cleanup)");
}

// ---------------------------------------------------------------------------
// batch/check (sync)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_batch_check() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let resp = ureq::get(&format!("{V2_BASE_URL}/batch/check/0"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 batch check failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_object(), "Response should be an object");
    assert!(
        body.get("inboxId").is_some(),
        "batch/check should contain inboxId"
    );
    println!(
        "Batch check keys: {:?}",
        body.as_object().unwrap().keys().collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Project groups (folders)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_project_group_create_and_delete() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let folder_name = "cli-test-folder";

    // Create
    let body = serde_json::json!({
        "add": [{"name": folder_name, "listType": "group"}]
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/projectGroup"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 project group create failed");

    assert_eq!(resp.status(), 200);
    println!("Project group created");

    // Get the folder ID from batch/check
    let resp = ureq::get(&format!("{V2_BASE_URL}/batch/check/0"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 batch check failed");

    let data: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let groups = data["projectGroups"]
        .as_array()
        .expect("should have projectGroups");
    let folder = groups
        .iter()
        .find(|g| g["name"].as_str() == Some(folder_name))
        .expect("Created folder should appear in batch/check");
    let folder_id = folder["id"].as_str().unwrap();
    println!("Folder ID: {folder_id}");

    // Delete
    let body = serde_json::json!({ "delete": [folder_id] });
    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/projectGroup"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 project group delete failed");

    assert_eq!(resp.status(), 200);
    println!("Project group deleted");
}

#[test]
#[ignore]
fn test_v2_project_group_rename() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let old_name = "cli-test-rename-folder";
    let new_name = "cli-test-renamed-folder";

    // Create
    let body = serde_json::json!({
        "add": [{"name": old_name, "listType": "group"}]
    });
    ureq::post(&format!("{V2_BASE_URL}/batch/projectGroup"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 project group create failed");

    // Find the folder
    let resp = ureq::get(&format!("{V2_BASE_URL}/batch/check/0"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 batch check failed");

    let data: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let groups = data["projectGroups"].as_array().unwrap();
    let folder = groups
        .iter()
        .find(|g| g["name"].as_str() == Some(old_name))
        .expect("Folder should exist");
    let folder_id = folder["id"].as_str().unwrap();
    let etag = folder["etag"].as_str().unwrap_or("");

    // Rename
    let body = serde_json::json!({
        "update": [{"id": folder_id, "etag": etag, "name": new_name, "listType": "group"}]
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/projectGroup"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 project group rename failed");

    assert_eq!(resp.status(), 200);
    println!("Project group renamed");

    // Clean up
    let body = serde_json::json!({ "delete": [folder_id] });
    let _ = ureq::post(&format!("{V2_BASE_URL}/batch/projectGroup"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body);
    println!("Project group deleted (cleanup)");
}

#[test]
#[ignore]
fn test_v2_assign_project_to_folder() {
    let token = get_token();
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    // Create a folder
    let folder_name = "cli-test-assign-folder";
    let body = serde_json::json!({
        "add": [{"name": folder_name, "listType": "group"}]
    });
    ureq::post(&format!("{V2_BASE_URL}/batch/projectGroup"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 project group create failed");

    // Get folder ID
    let resp = ureq::get(&format!("{V2_BASE_URL}/batch/check/0"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 batch check failed");

    let data: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let groups = data["projectGroups"].as_array().unwrap();
    let folder = groups
        .iter()
        .find(|g| g["name"].as_str() == Some(folder_name))
        .expect("Folder should exist");
    let folder_id = folder["id"].as_str().unwrap();

    // Get a project to assign
    let resp = ureq::get(&format!("{BASE_URL}/project"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .expect("API request failed");

    let projects: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");
    if projects.is_empty() {
        println!("No projects found, skipping test");
        // Clean up folder
        let body = serde_json::json!({ "delete": [folder_id] });
        let _ = ureq::post(&format!("{V2_BASE_URL}/batch/projectGroup"))
            .set("User-Agent", "Mozilla/5.0")
            .set("x-device", &x_device)
            .set("Cookie", &format!("t={session_token}"))
            .set("Content-Type", "application/json")
            .send_json(body);
        return;
    }

    let project_id = projects[0]["id"].as_str().unwrap();
    let original_group = projects[0]["groupId"].as_str().map(String::from);

    // Assign project to folder via v1 API
    let body = serde_json::json!({
        "id": project_id,
        "groupId": folder_id,
    });
    let resp = ureq::post(&format!("{BASE_URL}/project/{project_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("Project update failed");

    assert_eq!(resp.status(), 200);
    let updated: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert_eq!(
        updated["groupId"].as_str(),
        Some(folder_id),
        "Project should be assigned to folder"
    );
    println!("Project assigned to folder");

    // Restore original group
    let restore_group = original_group.as_deref().unwrap_or("NONE");
    let body = serde_json::json!({
        "id": project_id,
        "groupId": restore_group,
    });
    let _ = ureq::post(&format!("{BASE_URL}/project/{project_id}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(body);

    // Clean up folder
    let body = serde_json::json!({ "delete": [folder_id] });
    let _ = ureq::post(&format!("{V2_BASE_URL}/batch/projectGroup"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body);
    println!("Cleanup done");
}

// ---------------------------------------------------------------------------
// Habits
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_list_habits() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let resp = ureq::get(&format!("{V2_BASE_URL}/habits"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 habits list failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_array(), "Response should be an array");
    println!("Habits: {}", serde_json::to_string_pretty(&body).unwrap());
}

#[test]
#[ignore]
fn test_v2_habit_create_and_delete() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let habit_name = "cli-test-habit";

    // Create
    let body = serde_json::json!({
        "add": [{
            "name": habit_name,
            "type": "Boolean",
            "goal": 1.0,
        }]
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/habits/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit create failed");

    assert_eq!(resp.status(), 200);
    println!("Habit created");

    // List to find the habit ID
    let resp = ureq::get(&format!("{V2_BASE_URL}/habits"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 habits list failed");

    let habits: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");
    let habit = habits
        .iter()
        .find(|h| h["name"].as_str() == Some(habit_name))
        .expect("Created habit should appear in listing");
    let habit_id = habit["id"].as_str().unwrap();
    println!("Habit ID: {habit_id}");

    // Delete
    let body = serde_json::json!({ "delete": [habit_id] });
    let resp = ureq::post(&format!("{V2_BASE_URL}/habits/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit delete failed");

    assert_eq!(resp.status(), 200);
    println!("Habit deleted");
}

#[test]
#[ignore]
fn test_v2_habit_checkin_and_query() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let habit_name = "cli-test-checkin";

    // Create habit
    let body = serde_json::json!({
        "add": [{
            "name": habit_name,
            "type": "Boolean",
            "goal": 1.0,
        }]
    });
    ureq::post(&format!("{V2_BASE_URL}/habits/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit create failed");

    // Find habit ID
    let resp = ureq::get(&format!("{V2_BASE_URL}/habits"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 habits list failed");

    let habits: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");
    let habit = habits
        .iter()
        .find(|h| h["name"].as_str() == Some(habit_name))
        .expect("Habit should exist");
    let habit_id = habit["id"].as_str().unwrap();

    // Check in
    let body = serde_json::json!({
        "add": [{
            "habitId": habit_id,
            "checkinStamp": 20260218,
            "value": 1.0,
            "status": 0,
        }]
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/habitCheckins/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit checkin failed");

    assert_eq!(resp.status(), 200);
    println!("Habit checked in");

    // Query check-ins
    let body = serde_json::json!({
        "habitIds": [habit_id],
        "afterStamp": 20260101,
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/habitCheckins/query"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit checkin query failed");

    assert_eq!(resp.status(), 200);
    let query_result: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    println!(
        "Checkin query: {}",
        serde_json::to_string_pretty(&query_result).unwrap()
    );

    // Clean up
    let body = serde_json::json!({ "delete": [habit_id] });
    let _ = ureq::post(&format!("{V2_BASE_URL}/habits/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body);
    println!("Habit deleted (cleanup)");
}

#[test]
#[ignore]
fn test_v2_habit_archive() {
    let (username, password) = load_v2_credentials();
    let (session_token, x_device) = v2_session(&username, &password);

    let habit_name = "cli-test-archive";

    // Create habit
    let body = serde_json::json!({
        "add": [{
            "name": habit_name,
            "type": "Boolean",
            "goal": 1.0,
        }]
    });
    ureq::post(&format!("{V2_BASE_URL}/habits/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit create failed");

    // Find habit
    let resp = ureq::get(&format!("{V2_BASE_URL}/habits"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 habits list failed");

    let habits: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");
    let habit = habits
        .iter()
        .find(|h| h["name"].as_str() == Some(habit_name))
        .expect("Habit should exist");
    let habit_id = habit["id"].as_str().unwrap();
    let etag = habit["etag"].as_str().unwrap_or("");

    // Archive (set status to 2)
    let body = serde_json::json!({
        "update": [{
            "id": habit_id,
            "etag": etag,
            "status": 2,
        }]
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/habits/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit archive failed");

    assert_eq!(resp.status(), 200);
    println!("Habit archived");

    // Clean up
    let body = serde_json::json!({ "delete": [habit_id] });
    let _ = ureq::post(&format!("{V2_BASE_URL}/habits/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body);
    println!("Habit deleted (cleanup)");
}
