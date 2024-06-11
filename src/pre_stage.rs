use std::borrow::Cow;

use log::{debug, error, info};
use shlex::Shlex;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    chdir, cwd, get_action_input, node_exit, parse_gh_bool, set_action_output,
    spawn_cmd,
};

/// Pass in a complete command. Call shlex to extract the command into Some((cmd, args))
fn split_shell_cmd(raw: &str) -> Option<(String, Vec<String>)> {
    debug!("split_shell_cmd()");
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut cmd_arr = Shlex::new(trimmed);
    let cmd = cmd_arr.next()?;

    let args = cmd_arr.collect::<Vec<_>>();
    info!("cmd: {cmd}, args: {args:?}");
    Some((cmd, args))
}

#[wasm_bindgen]
pub fn run_pre_local_cmd() {
    debug!("run_pre_local_cmd()");
    let set_output = |status: bool| {
        set_action_output("pre-local-cmd-success", &status.to_string())
    };

    let raw_cmd = match cfg!(debug_assertions) {
        true => Cow::Borrowed("ls -la -h"),
        _ => {
            let s = get_action_input("pre-local-cmd");
            match s.trim() {
                "" => return set_output(false),
                _ => Cow::Owned(s),
            }
        }
    };
    debug!("raw: {raw_cmd}");

    let allow_failure = parse_gh_bool("allow-pre-local-cmd-failure", true);
    let task_async = parse_gh_bool("pre-local-cmd-async", true);

    let Some((cmd, args)) = split_shell_cmd(&raw_cmd) else {
        return set_output(false);
    };

    info!("running {cmd}...");

    if let Err(e) = spawn_cmd(&cmd, args, task_async) {
        error!("pre-local-cmd: {e:?}");
        set_output(false);
        if !allow_failure {
            node_exit(Some(1))
        }
        return;
    };
    set_output(true);
}

#[wasm_bindgen]
pub fn set_pre_local_workdir() {
    debug!("set_pre_local_workdir()");

    let raw = match cfg!(debug_assertions) {
        true => Cow::Borrowed("../"),
        _ => {
            let s = get_action_input("pre-local-workdir");
            match s.trim() {
                "" => return,
                _ => Cow::Owned(s),
            }
        }
    };

    info!("old local working directory: {}", cwd());
    chdir(&raw);
    info!("new local working directory: {}", cwd());
}
