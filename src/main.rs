//! git-remote-agent — `git remote add agent https://cocapn.dev/repo`
//! Push your branch to the agent remote, pull commits back.
//! Works with every git client. Zero install.
//!
//! Usage:
//!   git remote add agent https://cocapn.dev/your-repo
//!   git push agent main
//!   # agent processes, commits pushed back
//!   git pull agent main

use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let remote_url = env::var("GIT_REMOTE_HELPER").unwrap_or_else(|_| "agent".into());

    if args.len() < 3 {
        eprintln!("Usage: git-remote-agent <remote-name> <url>");
        eprintln!("  Installed as git remote helper for 'agent://' protocol");
        std::process::exit(1);
    }

    let remote_name = &args[1];
    let repo_url = &args[2];

    match args.get(2).map(|s| s.as_str()) {
        Some(_) => run_helper(remote_name, repo_url),
        None => {
            eprintln!("git-remote-agent: missing remote url");
            std::process::exit(1);
        }
    }
}

fn run_helper(remote_name: &str, repo_url: &str) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Git remote helper protocol: read commands from stdin
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "capabilities" => {
                writeln!(stdout, "push").unwrap();
                writeln!(stdout, "fetch").unwrap();
                writeln!(stdout, "option").unwrap();
                writeln!(stdout).unwrap();
            }
            "list" => {
                // List refs: HEAD and main
                let head_sha = get_head_sha();
                writeln!(stdout, "?{} HEAD", head_sha).unwrap();
                writeln!(stdout, "@{} HEAD refs/heads/main", head_sha).unwrap();
                writeln!(stdout).unwrap();
            }
            "option" => {
                // Handle option negotiation
                writeln!(stdout, "ok").unwrap();
            }
            "push" => {
                // Parse push ref: <local-sha> <local-ref> <remote-ref> <push-status>
                if parts.len() >= 4 {
                    let local_sha = parts[1];
                    let local_ref = parts[2];
                    let remote_ref = parts[3];

                    // Send to agent endpoint
                    match push_to_agent(repo_url, local_sha, local_ref, remote_ref) {
                        Ok(agent_sha) => {
                            writeln!(stdout, "ok {}", local_ref).unwrap();
                            writeln!(stdout, "").unwrap();

                            // Update the ref
                            if agent_sha != local_sha {
                                writeln!(stdout, "{} {}", agent_sha, remote_ref).unwrap();
                                writeln!(stdout, "").unwrap();
                            }
                        }
                        Err(e) => {
                            writeln!(stdout, "error {} agent processing failed: {}", local_ref, e).unwrap();
                            writeln!(stdout, "").unwrap();
                        }
                    }
                }
                writeln!(stdout, "done").unwrap();
                writeln!(stdout, "").unwrap();
            }
            "fetch" => {
                // Fetch refs from agent
                if parts.len() >= 3 {
                    let remote_ref = parts[1];
                    let local_sha = parts[2];

                    match fetch_from_agent(repo_url, remote_ref) {
                        Ok(agent_sha) => {
                            if agent_sha != local_sha {
                                writeln!(stdout, "ok {}", remote_ref).unwrap();
                                writeln!(stdout, "").unwrap();
                            }
                        }
                        Err(e) => {
                            eprintln!("fetch error: {}", e);
                        }
                    }
                }
                writeln!(stdout, "done").unwrap();
                writeln!(stdout, "").unwrap();
            }
            "" => break,
            _ => {
                // Unknown command, skip
            }
        }
        stdout.flush().unwrap();
    }
}

fn get_head_sha() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "0000000000000000000000000000000000000000".into())
}

fn push_to_agent(repo_url: &str, sha: &str, local_ref: &str, remote_ref: &str) -> Result<String, String> {
    // Get commit info
    let message = Command::new("git")
        .args(["log", "-1", "--format=%s", sha])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let diff = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "-r", "--name-status", sha])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // Call agent endpoint
    let agent_url = format!("{}/api/push", repo_url);

    // Use curl as http client (no dependencies)
    let body = serde_json::json!({
        "sha": sha,
        "ref": remote_ref,
        "message": message,
        "files": diff,
    })
    .to_string();

    let output = Command::new("curl")
        .args([
            "-s", "-X", "POST",
            &agent_url,
            "-H", "Content-Type: application/json",
            "-d", &body,
        ])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    if output.status.success() {
        let resp: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("parse error: {}", e))?;
        Ok(resp["sha"].as_str().unwrap_or(sha).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn fetch_from_agent(repo_url: &str, remote_ref: &str) -> Result<String, String> {
    let agent_url = format!("{}/api/fetch?ref={}", repo_url, remote_ref);

    let output = Command::new("curl")
        .args(["-s", &agent_url])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    if output.status.success() {
        let resp: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("parse error: {}", e))?;
        Ok(resp["sha"].as_str().unwrap_or("").to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
