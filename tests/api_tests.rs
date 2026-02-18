use std::fs;

struct EnvConfig {
    access_token: Option<String>,
    v2_session_token: Option<String>,
}

fn load_env() -> EnvConfig {
    let content = fs::read_to_string(".env").expect("Failed to read .env file");

    let mut client_id = None;
    let mut client_secret = None;
    let mut access_token = None;
    let mut v2_session_token = None;

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
                "TICKTICK_V2_SESSION_TOKEN" => v2_session_token = Some(value.to_string()),
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
        v2_session_token,
    }
}

fn get_token() -> String {
    let env = load_env();
    env.access_token
        .expect("TICKTICK_ACCESS_TOKEN not found in .env - required for API tests")
}

fn get_v2_session() -> (String, String) {
    let env = load_env();
    let token = env
        .v2_session_token
        .expect("TICKTICK_V2_SESSION_TOKEN not found in .env - required for v2 API tests");
    let device_id = "6490test00000000000000";
    let x_device = format!(
        r#"{{"platform":"web","os":"macOS 10.15.7","device":"Chrome 130.0.0.0","name":"","version":6490,"id":"{device_id}","channel":"website","campaign":"","websocket":""}}"#
    );
    (token, x_device)
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

    // Skip inbox projects (id starts with "inbox") — the v1 GET /project/{id} endpoint
    // returns no `name` field for the inbox. This is a v1 API quirk only; the inbox works
    // fine everywhere else (task CRUD, project listing, etc.).
    let project = projects
        .iter()
        .find(|p| p["id"].as_str().is_some_and(|id| !id.starts_with("inbox")));

    let Some(project) = project else {
        println!("No non-inbox projects found, skipping test");
        return;
    };

    let project_id = project["id"].as_str().unwrap();
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

const V2_BASE_URL: &str = "https://api.ticktick.com/api/v2";

#[test]
#[ignore]
fn test_v2_move_task_between_projects() {
    let token = get_token();
    let (session_token, x_device) = get_v2_session();

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

#[test]
#[ignore]
fn test_v2_list_completed_tasks() {
    let (session_token, x_device) = get_v2_session();

    // Note: from/to date params cause 500 errors on v2 session API, use limit only
    let resp = ureq::get(&format!(
        "{V2_BASE_URL}/project/all/completedInAll/?limit=10"
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
    let (session_token, x_device) = get_v2_session();

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
    let (session_token, x_device) = get_v2_session();

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
fn test_v2_tag_update() {
    let (session_token, x_device) = get_v2_session();

    let tag_name = "cli-test-update";

    // Create
    let body = serde_json::json!({ "add": [{ "label": tag_name, "name": tag_name }] });
    ureq::post(&format!("{V2_BASE_URL}/batch/tag"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 tag create failed");

    // Update color
    let body = serde_json::json!({ "update": [{ "name": tag_name, "color": "#FF0000" }] });
    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/tag"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 tag update failed");

    assert_eq!(resp.status(), 200);
    println!("Tag updated");

    // Clean up
    let _ = ureq::delete(&format!("{V2_BASE_URL}/tag?name={tag_name}"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call();
    println!("Tag deleted (cleanup)");
}

#[test]
#[ignore]
fn test_v2_tag_merge() {
    let (session_token, x_device) = get_v2_session();

    let source = "cli-test-merge-source";
    let target = "cli-test-merge-target";

    // Create both tags
    let body = serde_json::json!({ "add": [
        { "label": source, "name": source },
        { "label": target, "name": target },
    ] });
    ureq::post(&format!("{V2_BASE_URL}/batch/tag"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 tag create failed");

    // Merge source into target
    let merge_body = serde_json::json!({ "name": source, "newName": target });
    let resp = ureq::put(&format!("{V2_BASE_URL}/tag/merge"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(merge_body)
        .expect("v2 tag merge failed");

    assert_eq!(resp.status(), 200);
    println!("Tags merged");

    // Verify source is gone
    let resp = ureq::get(&format!("{V2_BASE_URL}/tags"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 tag list failed");

    let tags: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");
    let source_found = tags.iter().any(|t| t["name"].as_str() == Some(source));
    assert!(!source_found, "Source tag should be deleted after merge");
    println!("Source tag confirmed deleted");

    // Clean up target
    let _ = ureq::delete(&format!("{V2_BASE_URL}/tag?name={target}"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call();
    println!("Target tag deleted (cleanup)");
}

#[test]
#[ignore]
fn test_v2_set_task_parent() {
    let token = get_token();
    let (session_token, x_device) = get_v2_session();

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
// Calendar
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_calendar_accounts() {
    let (session_token, x_device) = get_v2_session();

    let resp = ureq::get(&format!("{V2_BASE_URL}/calendar/third/accounts"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 calendar accounts failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    println!(
        "Calendar accounts: {}",
        serde_json::to_string_pretty(&body).unwrap()
    );
}

#[test]
#[ignore]
fn test_v2_calendar_events() {
    let (session_token, x_device) = get_v2_session();

    let body = serde_json::json!({
        "begin": "2026-02-11T00:00:00.000+0000",
        "end": "2026-02-25T00:00:00.000+0000",
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/calendar/bind/events/all"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 calendar events failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    println!(
        "Calendar events: {}",
        serde_json::to_string_pretty(&body).unwrap()
    );
}

// ---------------------------------------------------------------------------
// Profile / Settings
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_profile() {
    let (session_token, x_device) = get_v2_session();

    let resp = ureq::get(&format!("{V2_BASE_URL}/user/profile"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 profile failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_object(), "Profile should be an object");
    println!("Profile: {}", serde_json::to_string_pretty(&body).unwrap());
}

#[test]
#[ignore]
fn test_v2_settings() {
    let (session_token, x_device) = get_v2_session();

    let resp = ureq::get(&format!(
        "{V2_BASE_URL}/user/preferences/settings?includeWeb=true"
    ))
    .set("User-Agent", "Mozilla/5.0")
    .set("x-device", &x_device)
    .set("Cookie", &format!("t={session_token}"))
    .call()
    .expect("v2 settings failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_object(), "Settings should be an object");
    println!("Settings: {}", serde_json::to_string_pretty(&body).unwrap());
}

// ---------------------------------------------------------------------------
// Trash
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_list_trash() {
    let (session_token, x_device) = get_v2_session();

    let resp = ureq::get(&format!("{V2_BASE_URL}/project/all/trash/page"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 trash list failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    assert!(body.is_object(), "Response should be an object");
    println!("Trash: {}", serde_json::to_string_pretty(&body).unwrap());
}

// ---------------------------------------------------------------------------
// batch/check (sync)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_batch_check() {
    let (session_token, x_device) = get_v2_session();

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
    let (session_token, x_device) = get_v2_session();

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
    let (session_token, x_device) = get_v2_session();

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
    let (session_token, x_device) = get_v2_session();

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

    // Get folder ID and projects from batch/check
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
    let projects = data["projectProfiles"].as_array().unwrap();
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

    // Assign project to folder via v2 batch/project
    let body = serde_json::json!({
        "update": [{
            "id": project_id,
            "groupId": folder_id,
        }]
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/project"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 batch project update failed");

    assert_eq!(resp.status(), 200);

    // Verify via v2 batch/check
    let resp = ureq::get(&format!("{V2_BASE_URL}/batch/check/0"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 batch check failed");

    let data: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let projects = data["projectProfiles"].as_array().unwrap();
    let updated = projects
        .iter()
        .find(|p| p["id"].as_str() == Some(project_id))
        .expect("Project should exist in batch check");
    assert_eq!(
        updated["groupId"].as_str(),
        Some(folder_id),
        "Project should be assigned to folder"
    );
    println!("Project assigned to folder");

    // Restore original group via v2
    let restore_group: serde_json::Value = match original_group.as_deref() {
        Some(gid) => serde_json::json!(gid),
        None => serde_json::Value::Null,
    };
    let body = serde_json::json!({
        "update": [{
            "id": project_id,
            "groupId": restore_group,
        }]
    });
    let _ = ureq::post(&format!("{V2_BASE_URL}/batch/project"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
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
// Filters
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_filter_create_and_delete() {
    let (session_token, x_device) = get_v2_session();

    let filter_name = "cli-test-filter";
    let rule =
        r#"{"and":[{"conditionName":"priority","or":[5],"conditionType":1}],"type":0,"version":1}"#;

    // Create
    let body = serde_json::json!({
        "add": [{
            "name": filter_name,
            "rule": rule,
            "sortType": "dueDate",
        }]
    });
    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/filter"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 filter create failed");

    assert_eq!(resp.status(), 200);
    println!("Filter created");

    // Find the filter ID via batch/check
    let resp = ureq::get(&format!("{V2_BASE_URL}/batch/check/0"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 batch check failed");

    let data: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    let filters = data["filters"].as_array().expect("should have filters");
    let filter = filters
        .iter()
        .find(|f| f["name"].as_str() == Some(filter_name))
        .expect("Created filter should appear in batch/check");
    let filter_id = filter["id"].as_str().unwrap();
    println!("Filter ID: {filter_id}");

    // Delete
    let body = serde_json::json!({ "delete": [filter_id] });
    let resp = ureq::post(&format!("{V2_BASE_URL}/batch/filter"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 filter delete failed");

    assert_eq!(resp.status(), 200);
    println!("Filter deleted");
}

// ---------------------------------------------------------------------------
// Habits
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_list_habits() {
    let (session_token, x_device) = get_v2_session();

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
    let (session_token, x_device) = get_v2_session();

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
    let (session_token, x_device) = get_v2_session();

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
    let (session_token, x_device) = get_v2_session();

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

    // Archive (set status to 1)
    let body = serde_json::json!({
        "update": [{
            "id": habit_id,
            "etag": etag,
            "status": 1,
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

// ---------------------------------------------------------------------------
// Habit Sections
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_habit_section_crud() {
    let (session_token, x_device) = get_v2_session();

    let section_name = "cli-test-section";

    // Create
    let body = serde_json::json!({ "add": [{ "name": section_name }] });
    let resp = ureq::post(&format!("{V2_BASE_URL}/habitSections/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit section create failed");

    assert_eq!(resp.status(), 200);
    println!("Section created");

    // List to find ID
    let resp = ureq::get(&format!("{V2_BASE_URL}/habitSections"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 habit sections list failed");

    let sections: Vec<serde_json::Value> = resp.into_json().expect("Failed to parse JSON");
    let section = sections
        .iter()
        .find(|s| s["name"].as_str() == Some(section_name))
        .expect("Created section should appear in listing");
    let section_id = section["id"].as_str().unwrap();
    println!("Section ID: {section_id}");

    // Rename
    let new_name = "cli-test-section-renamed";
    let body = serde_json::json!({ "update": [{ "id": section_id, "name": new_name }] });
    let resp = ureq::post(&format!("{V2_BASE_URL}/habitSections/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit section rename failed");

    assert_eq!(resp.status(), 200);
    println!("Section renamed");

    // Delete
    let body = serde_json::json!({ "delete": [section_id] });
    let resp = ureq::post(&format!("{V2_BASE_URL}/habitSections/batch"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .expect("v2 habit section delete failed");

    assert_eq!(resp.status(), 200);
    println!("Section deleted");
}

// ---------------------------------------------------------------------------
// Focus / Pomodoro (read-only)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_v2_focus_timer_status() {
    let (session_token, x_device) = get_v2_session();

    let resp = ureq::get(&format!("{V2_BASE_URL}/timer"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 timer status failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    println!(
        "Timer status: {}",
        serde_json::to_string_pretty(&body).unwrap()
    );
}

#[test]
#[ignore]
fn test_v2_focus_stats() {
    let (session_token, x_device) = get_v2_session();

    let resp = ureq::get(&format!(
        "{V2_BASE_URL}/pomodoros/statistics/generalForDesktop"
    ))
    .set("User-Agent", "Mozilla/5.0")
    .set("x-device", &x_device)
    .set("Cookie", &format!("t={session_token}"))
    .call()
    .expect("v2 focus stats failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    println!(
        "Focus stats: {}",
        serde_json::to_string_pretty(&body).unwrap()
    );
}

#[test]
#[ignore]
fn test_v2_focus_timeline() {
    let (session_token, x_device) = get_v2_session();

    let resp = ureq::get(&format!("{V2_BASE_URL}/pomodoros/timeline"))
        .set("User-Agent", "Mozilla/5.0")
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={session_token}"))
        .call()
        .expect("v2 focus timeline failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().expect("Failed to parse JSON");
    println!(
        "Focus timeline: {}",
        serde_json::to_string_pretty(&body).unwrap()
    );
}
