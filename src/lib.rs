use std::borrow::Cow;

use clg::{
    clg,
    log_level::{new_log_level, LogLevel},
    ConsoleLogger,
};
use log::{debug, error, info};
use shlex::Shlex;

use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
mod wasm;

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct InputConfig {
    // log_level: clg::log_level::LogLevel,
    // pre_local_cmd: String,
    // pre_local_workdir: String,
    ssh_bin: String,
    args: Option<Vec<String>>,
    host: String,
    pre_exit_cmd: Vec<String>,
    pre_sleep: u32,
    pre_timeout: u32,
    run: String,
    allow_run_failure: bool,
    post_run: Option<String>,
    allow_post_run_failure: bool,
}

#[wasm_bindgen(module = "@actions/core")]
extern "C" {
    #[wasm_bindgen(js_name = getInput)]
    fn get_action_input(s: &str) -> String;

    // #[wasm_bindgen(js_name = getBooleanInput)]
    // fn get_bool_input(s: &str) -> bool;

    #[wasm_bindgen(js_name = setOutput)]
    fn set_action_output(key: &str, value: &str);
}

#[wasm_bindgen(module = "node:process")]
extern "C" {
    #[wasm_bindgen]
    fn cwd() -> String;

    #[wasm_bindgen]
    fn chdir(s: &str);
}

#[wasm_bindgen(raw_module = "../js/ffi.cjs")]
extern "C" {
    #[wasm_bindgen(js_name = spawnCmd, catch)]
    fn spawn_cmd(cmd: &str, args: Vec<String>, sync: bool) -> Result<(), JsValue>;

    #[wasm_bindgen]
    fn sleep(time: u32);

    #[wasm_bindgen(js_name = nodeExit)]
    fn node_exit(code: Option<u8>);

    #[wasm_bindgen(js_name = retryCommandWithinTime)]
    fn retry_command_within_time(cmd: &str, args: Vec<String>, timeout: u32)
        -> bool;
}

#[wasm_bindgen]
pub fn init_logger() -> ConsoleLogger {
    let init = ConsoleLogger::init;

    let raw_lv = get_action_input("log-level");
    let opt_lv = match raw_lv.trim() {
        #[cfg(debug_assertions)]
        "" => Some(LogLevel::Debug),
        #[cfg(not(debug_assertions))]
        "" => Some(LogLevel::Info),
        lv => new_log_level(lv),
    };

    init(opt_lv)
}

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

    let raw_cmd = match cfg!(debug_assertions) {
        true => Cow::Borrowed("ls -la -h"),
        _ => {
            let s = get_action_input("pre-local-cmd");
            match s.trim() {
                "" => return,
                _ => Cow::Owned(s),
            }
        }
    };
    debug!("raw: {raw_cmd}");

    let allow_failure = parse_gh_bool("allow-pre-local-cmd-failure", true);
    let task_async = parse_gh_bool("pre-local-cmd-async", true);

    let set_output = |status: bool| {
        set_action_output("pre-local-cmd-success", &status.to_string())
    };

    let Some((cmd, args)) = split_shell_cmd(&raw_cmd) else {
        return set_output(false);
    };

    info!("running {cmd}...");
    spawn_cmd(&cmd, args, task_async).unwrap_or_else(|e| {
        error!("pre-local-cmd: {e:?}");
        set_output(false);
        if !allow_failure {
            node_exit(Some(1))
        }
    });
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

fn not_empty_or(owned_raw: String, dft: &str) -> Cow<str> {
    match owned_raw.trim() {
        "" => Cow::Borrowed(dft),
        _ => Cow::Owned(owned_raw),
    }
}

fn parse_gh_bool(input: &str, dft: bool) -> bool {
    match get_action_input(input).trim() {
        "" => dft,
        x => match x.parse() {
            Ok(b) => b,
            _ => match x.to_ascii_lowercase().as_ref() {
                "false" | "n" | "no" | "off" | "err" => false,
                "true" | "y" | "yes" | "on" | "ok" => true,
                _ => dft,
            },
        },
    }
}

fn parse_gh_num(input: &str) -> u32 {
    match get_action_input(input).trim() {
        "" => 0,
        x => x.parse().unwrap_or(0),
    }
}

#[wasm_bindgen]
pub fn get_main_input_config() -> InputConfig {
    let wrap_opt = |owned_raw: String| match owned_raw.trim() {
        "" => None,
        _ => Some(owned_raw),
    };

    let gh = get_action_input;

    let ssh_bin = not_empty_or(gh("ssh-bin"), "ssh").into_owned();

    let args = wrap_opt(gh("args")).map(|x| Shlex::new(&x).collect::<Vec<_>>());

    let host = not_empty_or(gh("host"), "127.0.0.1").into_owned();

    let pre_exit_cmd = match not_empty_or(gh("pre-exit-cmd"), "exit").as_ref() {
        "exit" => ["exit".into()].into(),
        x => Shlex::new(x).collect::<Vec<_>>(),
    };

    let pre_sleep = parse_gh_num("pre-sleep");
    let pre_timeout = parse_gh_num("pre-timeout");
    let run = gh("run");
    let allow_run_failure = parse_gh_bool("allow-run-failure", false);

    let post_run = wrap_opt(gh("post-run"));
    let allow_post_run_failure = parse_gh_bool("allow-post-run-failure", true);

    InputConfig {
        ssh_bin,
        args,
        host,
        pre_exit_cmd,
        pre_sleep,
        pre_timeout,
        run,
        allow_run_failure,
        post_run,
        allow_post_run_failure,
    }
}

#[wasm_bindgen]
pub fn ssh_connection(cfg: &InputConfig) {
    debug!("pre_timeout_connection()");

    let InputConfig {
        ssh_bin,
        args,
        host,
        pre_exit_cmd,
        pre_sleep,
        pre_timeout,
        run,
        allow_run_failure,
        post_run,
        allow_post_run_failure,
    } = cfg;
    debug!("{cfg:?}");

    if *pre_sleep > 0 {
        info!("sleep: {pre_sleep}s");
        sleep(*pre_sleep);
    }

    let mut real_args = Vec::with_capacity(32);
    {
        if let Some(arg) = args {
            real_args.extend(arg.iter().cloned())
        }
        real_args.push(host.into());
    }

    if pre_timeout > &0 {
        let mut pre_args: Vec<String> = real_args.clone();
        pre_args.extend(pre_exit_cmd.iter().cloned());

        debug!("pre_args: {pre_args:?}");
        info!("pre_timeout: {pre_timeout:?}s");
        let pre_status = retry_command_within_time(ssh_bin, pre_args, *pre_timeout);

        if !pre_status {
            error!("Failed to connect to ssh, ssh_bin: {ssh_bin}");
            node_exit(Some(1))
        }
    }

    {
        let mut main_args = real_args.clone();
        match run.trim() {
            "" => (),
            _ => main_args.push(run.into()),
        }
        debug!("main_args: {main_args:?}");
        let mut status = true;
        spawn_cmd(ssh_bin, main_args, true).unwrap_or_else(|e| {
            error!("Main-Task: {e:?}");
            status = false;
            if !allow_run_failure {
                node_exit(Some(1))
            }
        });
        set_action_output("main-run-success", &status.to_string());
    }

    {
        if let Some(r) = post_run {
            real_args.push(r.into());
            debug!("post_args: {real_args:?}");
            let mut status = true;

            spawn_cmd(ssh_bin, real_args, true).unwrap_or_else(|e| {
                error!("Post-Run: {e:?}");
                status = false;
                if !allow_post_run_failure {
                    node_exit(Some(1))
                }
            });
            set_action_output("post-run-success", &status.to_string());
        }
    }
}

#[wasm_bindgen]
pub fn test_sleep_2s() {
    sleep(2);
    clg!("OK")
}
