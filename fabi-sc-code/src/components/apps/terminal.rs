//! Terminal App Component
//!
//! A simple terminal emulator that executes VFS commands.

use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, KeyboardEvent};
use yew::prelude::*;

use crate::filesystem as vfs;
use wasm_bindgen_futures::spawn_local;

#[derive(Properties, PartialEq)]
pub struct TerminalAppProps {
    #[prop_or_default]
    pub window_id: String,
}

/// A single line of terminal output
#[derive(Clone, PartialEq)]
struct OutputLine {
    content: String,
    is_command: bool,
}

#[function_component(TerminalApp)]
pub fn terminal_app(_props: &TerminalAppProps) -> Html {
    let cwd = use_state(|| "/home".to_string());
    let output = use_state(Vec::<OutputLine>::new);
    let input_value = use_state(String::new);
    let history = use_state(Vec::<String>::new);
    let history_index = use_state(|| 0usize);

    // Handle input change
    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(target) = e.target() {
                if let Some(input) = target.dyn_ref::<HtmlInputElement>() {
                    input_value.set(input.value());
                }
            }
        })
    };

    // Handle key press (Enter to execute, Up/Down for history)
    let on_keydown = {
        let cwd = cwd.clone();
        let output = output.clone();
        let input_value = input_value.clone();
        let history = history.clone();
        let history_index = history_index.clone();
        Callback::from(move |e: KeyboardEvent| {
            match e.key().as_str() {
                "Enter" => {
                    let cmd = (*input_value).clone();
                    if cmd.is_empty() {
                        return;
                    }

                    // Add to history
                    let mut new_history = (*history).clone();
                    new_history.push(cmd.clone());
                    history.set(new_history.clone());
                    history_index.set(new_history.len());

                    // Add command to output
                    let mut new_output = (*output).clone();
                    new_output.push(OutputLine {
                        content: format!("{}$ {}", *cwd, cmd),
                        is_command: true,
                    });

                    // Execute command
                    let cwd_clone = cwd.clone();
                    let output_clone = output.clone();
                    let parts: Vec<&str> = cmd.split_whitespace().collect();

                    if !parts.is_empty() {
                        let command = parts[0];
                        let args: Vec<&str> = parts[1..].to_vec();
                        let cwd_val = (*cwd_clone).clone();

                        spawn_local(async move {
                            let result = execute_command(command, &args, &cwd_val).await;
                            let mut out = (*output_clone).clone();

                            // Add command line first if not already added
                            if out.last().map(|l| !l.is_command).unwrap_or(true) {
                                out.push(OutputLine {
                                    content: format!("{}$ {}", cwd_val, cmd),
                                    is_command: true,
                                });
                            }

                            match result {
                                CommandResult::Output(lines) => {
                                    for line in lines {
                                        out.push(OutputLine {
                                            content: line,
                                            is_command: false,
                                        });
                                    }
                                }
                                CommandResult::ChangeDir(new_cwd) => {
                                    cwd_clone.set(new_cwd);
                                }
                                CommandResult::Error(msg) => {
                                    out.push(OutputLine {
                                        content: msg,
                                        is_command: false,
                                    });
                                }
                                CommandResult::Clear => {
                                    out.clear();
                                }
                            }

                            output_clone.set(out);
                        });
                    }

                    output.set(new_output);
                    input_value.set(String::new());
                }
                "ArrowUp" => {
                    e.prevent_default();
                    if *history_index > 0 {
                        let new_idx = *history_index - 1;
                        history_index.set(new_idx);
                        if let Some(cmd) = history.get(new_idx) {
                            input_value.set(cmd.clone());
                        }
                    }
                }
                "ArrowDown" => {
                    e.prevent_default();
                    if *history_index < history.len() {
                        let new_idx = *history_index + 1;
                        history_index.set(new_idx);
                        if new_idx < history.len() {
                            if let Some(cmd) = history.get(new_idx) {
                                input_value.set(cmd.clone());
                            }
                        } else {
                            input_value.set(String::new());
                        }
                    }
                }
                _ => {}
            }
        })
    };

    html! {
        <div class="terminal-app">
            <div class="terminal-output">
                { for output.iter().map(|line| {
                    let class = if line.is_command { "terminal-line command" } else { "terminal-line" };
                    html! {
                        <div class={class}>{&line.content}</div>
                    }
                })}
            </div>
            <div class="terminal-input-line">
                <span class="terminal-prompt">{format!("{}$ ", *cwd)}</span>
                <input
                    type="text"
                    class="terminal-input"
                    value={(*input_value).clone()}
                    oninput={on_input}
                    onkeydown={on_keydown}
                    autofocus=true
                    autocomplete="off"
                    spellcheck="false"
                />
            </div>
        </div>
    }
}

/// Result of executing a command
enum CommandResult {
    Output(Vec<String>),
    ChangeDir(String),
    Error(String),
    Clear,
}

/// Execute a terminal command
async fn execute_command(cmd: &str, args: &[&str], cwd: &str) -> CommandResult {
    match cmd {
        "ls" => {
            let path = args.first().map(|s| resolve_path(s, cwd)).unwrap_or_else(|| cwd.to_string());
            match vfs::read_dir(&path).await {
                Ok(entries) => {
                    let lines: Vec<String> = entries.iter().map(|e| {
                        if e.is_dir() {
                            format!("{}/", e.name)
                        } else {
                            e.name.clone()
                        }
                    }).collect();
                    if lines.is_empty() {
                        CommandResult::Output(vec!["(empty)".to_string()])
                    } else {
                        CommandResult::Output(lines)
                    }
                }
                Err(e) => CommandResult::Error(format!("ls: {}", e)),
            }
        }
        "cd" => {
            let path = args.first().map(|s| resolve_path(s, cwd)).unwrap_or_else(|| "/home".to_string());
            match vfs::exists(&path).await {
                Ok(true) => {
                    // Check if it's a directory
                    match vfs::stat(&path).await {
                        Ok(node) if node.is_dir() => CommandResult::ChangeDir(path),
                        Ok(_) => CommandResult::Error(format!("cd: {}: Not a directory", path)),
                        Err(e) => CommandResult::Error(format!("cd: {}", e)),
                    }
                }
                Ok(false) => CommandResult::Error(format!("cd: {}: No such directory", path)),
                Err(e) => CommandResult::Error(format!("cd: {}", e)),
            }
        }
        "pwd" => {
            CommandResult::Output(vec![cwd.to_string()])
        }
        "cat" => {
            if args.is_empty() {
                return CommandResult::Error("cat: missing file operand".to_string());
            }
            let path = resolve_path(args[0], cwd);
            match vfs::read_to_string(&path).await {
                Ok(content) => {
                    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                    CommandResult::Output(lines)
                }
                Err(e) => CommandResult::Error(format!("cat: {}: {}", args[0], e)),
            }
        }
        "mkdir" => {
            if args.is_empty() {
                return CommandResult::Error("mkdir: missing operand".to_string());
            }
            let path = resolve_path(args[0], cwd);
            match vfs::create_dir(&path).await {
                Ok(_) => CommandResult::Output(vec![]),
                Err(e) => CommandResult::Error(format!("mkdir: {}", e)),
            }
        }
        "rm" => {
            if args.is_empty() {
                return CommandResult::Error("rm: missing operand".to_string());
            }
            let path = resolve_path(args[0], cwd);
            match vfs::remove_file(&path).await {
                Ok(_) => CommandResult::Output(vec![]),
                Err(e) => CommandResult::Error(format!("rm: {}", e)),
            }
        }
        "touch" => {
            if args.is_empty() {
                return CommandResult::Error("touch: missing operand".to_string());
            }
            let path = resolve_path(args[0], cwd);
            // Create empty file
            match vfs::write_file(&path, &[]).await {
                Ok(_) => CommandResult::Output(vec![]),
                Err(e) => CommandResult::Error(format!("touch: {}", e)),
            }
        }
        "echo" => {
            let text = args.join(" ");
            CommandResult::Output(vec![text])
        }
        "clear" => {
            CommandResult::Clear
        }
        "help" => {
            CommandResult::Output(vec![
                "Available commands:".to_string(),
                "  ls [path]     - List directory contents".to_string(),
                "  cd [path]     - Change directory".to_string(),
                "  pwd           - Print working directory".to_string(),
                "  cat <file>    - Display file contents".to_string(),
                "  mkdir <dir>   - Create directory".to_string(),
                "  rm <file>     - Remove file".to_string(),
                "  touch <file>  - Create empty file".to_string(),
                "  echo <text>   - Print text".to_string(),
                "  clear         - Clear terminal".to_string(),
                "  help          - Show this help".to_string(),
            ])
        }
        _ => {
            CommandResult::Error(format!("{}: command not found", cmd))
        }
    }
}

/// Resolve a path relative to cwd
fn resolve_path(path: &str, cwd: &str) -> String {
    if path.starts_with('/') {
        // Absolute path
        vfs::path::normalize(path)
    } else if path == ".." {
        // Parent directory
        vfs::path::parent(cwd).unwrap_or_else(|| "/".to_string())
    } else if path == "." {
        cwd.to_string()
    } else if path.starts_with("../") {
        let parent = vfs::path::parent(cwd).unwrap_or_else(|| "/".to_string());
        resolve_path(&path[3..], &parent)
    } else {
        // Relative path
        vfs::path::join(cwd, path)
    }
}
