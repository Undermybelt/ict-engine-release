use std::path::Path;

use super::repo_manager::is_git_repo;

pub fn required_files() -> Vec<String> {
    vec![
        "README.md".to_string(),
        "program.md".to_string(),
        "prepare.py".to_string(),
        "run.py".to_string(),
        "versions/README.md".to_string(),
    ]
}

pub fn verify_checkout(dir: &Path) -> (bool, Vec<String>) {
    let mut notes = Vec::new();
    let mut healthy = true;
    if !is_git_repo(dir) {
        healthy = false;
        notes.push("managed_dir_is_not_a_git_repo".to_string());
    }
    for relative in required_files() {
        if !dir.join(&relative).exists() {
            healthy = false;
            notes.push(format!("missing_required_file={relative}"));
        }
    }
    (healthy, notes)
}
