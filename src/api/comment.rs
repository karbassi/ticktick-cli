use serde::{Deserialize, Serialize};

/// A task comment as returned by the v1 API.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    #[serde(default)]
    pub user_id: Option<i64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub created_time: Option<i64>,
    #[serde(default)]
    pub modified_time: Option<i64>,
}

fn list_endpoint(project_id: &str, task_id: &str) -> String {
    format!("/project/{project_id}/task/{task_id}/comments")
}

fn add_endpoint(project_id: &str, task_id: &str) -> String {
    format!("/project/{project_id}/task/{task_id}/comment")
}

fn delete_endpoint(project_id: &str, task_id: &str, comment_id: &str) -> String {
    format!("/project/{project_id}/task/{task_id}/comment/{comment_id}")
}

fn add_body(text: &str) -> serde_json::Value {
    serde_json::json!({ "title": text })
}

/// List the comments on a task.
pub fn list(token: &str, project_id: &str, task_id: &str) -> Result<Vec<Comment>, String> {
    let resp = super::get(&list_endpoint(project_id, task_id), token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse comments: {e}"))
}

/// Add a comment to a task.
pub fn add(token: &str, project_id: &str, task_id: &str, text: &str) -> Result<Comment, String> {
    let resp = super::post(&add_endpoint(project_id, task_id), token, &add_body(text))?;
    resp.into_json()
        .map_err(|e| format!("failed to parse comment: {e}"))
}

/// Delete a comment from a task.
pub fn delete(
    token: &str,
    project_id: &str,
    task_id: &str,
    comment_id: &str,
) -> Result<(), String> {
    super::delete(&delete_endpoint(project_id, task_id, comment_id), token)?;
    Ok(())
}
