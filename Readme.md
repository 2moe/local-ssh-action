# Local SSH Action

| Languages/語言                                 | ID         |
| ---------------------------------------------- | ---------- |
| English                                        | en-Latn-US |
| [中文](./docs/Readme-zh.md)                    | zh-Hans-CN |
| [中文 (Traditional)](./docs/Readme-zh-Hant.md) | zh-Hant-TW |

Unlike other ssh actions, this one depends on local ssh.

It is more suitable for **self-hosted** runners.

On Ubuntu, if you don't have an ssh client, you will need to install `openssh-client` manually.

## What was the original intention of creating this action?

The original intention was to facilitate the connection of virtual machine.

Because some platform's cross-compilation toolchains are a bit tricky to handle, even if you get it done, setting up a test environment might not be as simple as you think.

For example, if I want to build a rust bin crate (binary package) for OpenBSD riscv64 on Linux arm64, then using the conventional method would be very tough.
But opening a virtual machine directly is much easier, you don't need to understand many details, and you can get started right away.

Initially, it was just a few lines of simple javascript scripts.
Later, based on my experience using ssh on github actions, I rewrote the core logic in rust and added many useful options.

I believe it will bring you a good ssh action experience.

## Inputs

| Inputs                      | Description                                                                                                       | Default   |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------- | --------- |
| log-level                   | optional values: "trace", "debug", "info", "warn", "error", "off"                                                 | info      |
| pre-local-workdir           | Localhost working directory                                                                                       |           |
| pre-local-cmd               | Execute the command through NodeJS's `spawn()` or `spawnSync()` before connecting to ssh.                         |           |
| pre-local-cmd-async         | Type: boolean. When true, the command is run asynchronously.                                                      | true      |
| allow-pre-local-cmd-failure | When true, ignore the errors of pre-local-cmd.                                                                    | false     |
| pre-sleep                   | blocking for a specific time before connecting to ssh, in seconds.                                                | 0         |
| pre-timeout                 | Since the ssh connection may fail, specifying pre-timeout allows it to keep trying to connect until it times out. | 0         |
| pre-exit-cmd                | The command needed to test ssh.                                                                                   | exit      |
| host                        | Remote Server Host                                                                                                | 127.0.0.1 |
| ssh-bin                     | In an unstable network environment, you may need to use a specific ssh, not the openssh client.                   | ssh       |
| run                         | Commands to be executed on remote server                                                                          |           |
| allow-run-failure           | Type: boolean                                                                                                     | false     |
| post-run                    | After the main `run` is completed, you can continue to run `post-run`                                             |           |
| allow-post-run-failure      | Type: boolean                                                                                                     | true      |
| args                        | SSH Client args. You can enter shell arguments for ssh, such as `-q`                                              |           |

## Get Started

Let's start with a simple example.

```yaml
name: test
on: push
jobs:
  test:
    runs-on: self-hosted
    steps:
      - id: ssh-action
        uses: 2moe/local-ssh-action@v0
        with:
          log-level: debug
          pre-local-cmd: printf "pre-local-cmd is running on the local host\n"
          pre-timeout: 20
          args: -q
          host: android-mobile
          run: |
            printf "This is on the remote host\n"
            /system/bin/toybox uname -m
          allow-run-failure: true
          post-run: printf "Bye\n"
```

You may not understand the content above, and this workflow may fail.

Don't worry, let me explain it to you slowly.

The example above is essentially executing `ssh android-mobile uname -m`. If there is no ssh configuration for `android-mobile` on the machine running actions, then it cannot connect.

The solution is simple, just create a configuration.

```yaml
name: test
on: push
jobs:
  test:
    runs-on: self-hosted
    steps:
      - name: create ssh config
        shell: sh -ex {0}
        run: |
          printf '
          Host android-mobile
            Hostname 192.168.123.234
            User shell
            Port 8222
            IdentityFile ~/.ssh/key/mobile_ed25519
            StrictHostKeyChecking no
          ' > cfg
          install -Dm600 cfg ~/.ssh/config.d/mobile.sshconf
          printf "%s\n" 'Include config.d/*.sshconf' >> ~/.ssh/config

      - id: ssh-action
        uses: 2moe/local-ssh-action@v0
        with:
          log-level: warn
          host: android-mobile
          run: uname -m
```

The configuration only needs to be created once. If the self-hosted machine has already created the android-mobile configuration, then there is no need to create it again before the next connection.

If you don't want to create a configuration, don't want to use the ed25519 key, and want to connect with a password in a single step, that can also be done.

But before that, we need to understand its basic usage first.

## Detailed Explanation

The `with` can accept many options, such as:

```yaml
with:
  log-level: debug
```

This section will provide a detailed explanation of these options.

### Core

Before expanding the explanation, you need to understand a core element: the process is carried out in stages.

They are:

- Pre
- Main
- Post

The formal stage of the ssh connection is Main, before the connection is Pre, and after the connection is Post.

Q: Why divide it into stages?

A:

- Because the ssh connection may fail before it is established. Therefore, in the Pre stage, you can keep trying to connect within a specified time until the connection is successful.
- After the ssh connection is completed, you may need to perform some cleanup tasks (e.g., shut down the virtual machine)
  - The cleanup task may fail, separating main and post can handle the task status separately.
    - i.e., main failure is not allowed, but post failure is allowed.

### Pre Stage

#### pre-local-workdir

```yaml
with:
  pre-local-workdir: /path/to/local-dir
```

- Type: String (file path)

This option modifies the local working directory, not the remote ssh directory.

Q: Why is there this option?

A: Suppose the ssh configuration file is not in **~/.ssh**, but is located in a specific directory. By specifying the directory before connecting to ssh, some operations can be simplified.

#### pre-local-cmd

```yaml
with:
  pre-local-cmd: ls -lah
```

- Type: String

Before connecting to ssh, execute commands through NodeJS's `spawn()` or `spawnSync()`.

> This is running on the local host, not the remote host.

Suppose `pre-local-cmd: ls -la -h` and `pre-local-cmd-async` is not configured, then it will automatically be parsed as `spawn("ls", ["-la", "-h"])`

#### pre-local-cmd-async

```yaml
with:
  pre-local-cmd-async: true
```

- Type: `bool`
- Default: `true`

- When it's true, the command runs asynchronously.
  - That is, before connecting to the remote ssh, let the local task run in the background.
- When it's false, the command runs synchronously (blocking).
  - Before connecting to the remote ssh, you must wait for the pre-local-cmd task to complete before you can continue to connect to ssh.

The default is true, i.e., it is asynchronous by default.

#### allow-pre-local-cmd-failure

```yaml
with:
  allow-pre-local-cmd-failure: true
```

- Type: `bool`
- Default: `true`

- When it's true, ignore the errors of pre-local-cmd.
  - More accurately, when pre-local-cmd fails, it will not cause the current step to crash.
- When it's false, if pre-local-cmd errors out, then this step will exit abnormally.

#### pre-sleep

```yaml
with:
  pre-sleep: 0
```

- Typescript type: number
- Rust type: u32
- Default: 0

Before connecting to ssh, synchronously (blocking) for a specific time, in seconds.

Suppose you want to connect to a machine that is currently restarting, if you connect now, it may disconnect after a few seconds.

At this time, you need to forcibly block, wait a few seconds, let it completely shut down, and then try to connect.

- Examples:
  - `pre-sleep: 1`, block for 1 second.
  - `pre-sleep: 30`, block for 30 seconds.

The reason for emphasizing the rust type is because it is parsed through the following function in the internal implementation.

```rust
fn parse_gh_num(input: &str) -> u32 {
    match get_action_input(input).trim() {
        "" => 0,
        x => x.parse().unwrap_or(0),
    }
}
```

u32 must be `>=0`, i.e., you cannot use `pre_sleep: -1` to represent infinite blocking.

P.S. If you need to test whether you can connect normally within a specified time, please use `pre-timeout`, not `pre-sleep`.

#### pre-timeout

```yaml
with:
  pre-timeout: 0
```

- Type: u32
- Default: 0

Because the ssh connection may fail, specifying pre-timeout allows you to wait for a specific time.

Unlike the blocking pre-sleep, for pre-timeout, once the test connection is successful, it will exit waiting.

- Examples:
  - `pre-timeout: 120`, the waiting timeout is 120 seconds.

Suppose you want to connect to a virtual machine that is currently booting up, then you have to wait for it to connect to the network and start the `sshd` process before you can connect to it.

If `pre-timeout: 30`, and the boot time of the virtual machine + the time to start sshd is 10 seconds, then it will not wait for 30 seconds, at 10+ seconds, once the test connection is successful, it will exit waiting.

#### pre-exit-cmd

```yaml
with:
  pre-exit-cmd: exit
```

- Type: String
- Default: "exit"

The command needed to test the ssh connection.

This option will only take effect when `pre-timeout > 0`.

Suppose `pre-timeout: 20`, `pre-exit-cmd: exit`, `host: netbsd-vm`

Then it will keep performing the `ssh netbsd-vm exit` connection test within 20 seconds.

Only after the test connection is successful, will it proceed to the next step.

If it fails after 20 seconds, then the entire step will fail.

### Shared in Multiple Stages

#### log-level

```yaml
with:
  log-level: debug
```

- Type: `enum LogLevel`
- Default: `info`
- Optional values: "trace", "debug", "info", "warn", "error", "off"

Among them, trace is the most detailed, debug is the second most detailed, and off has no logs.

#### host

```yaml
with:
  host: "127.0.0.1"
```

- Type: String
- Default: "127.0.0.1"

Remote host name or IP address.

#### ssh-bin

```yaml
with:
  ssh-bin: ssh
```

- Type: String
- Default: "ssh"

In an unstable network environment, you may need to use a specific ssh, not the openssh client.

As long as the command syntax conforms to the `{ssh-bin} {args} {host} {run}` rule, anything can be used.

Suppose there is `adb -s android-14 shell [run]`, then you can use

```yaml
ssh-bin: adb
args: -s android-14
host: shell
run: |
  ls -lh
  toybox printf "I am Android\n"
```

Suppose you need to automatically enter the password, then `sshpass -p $passwd ssh 192.168.50.10 [run]` can be converted to:

```yaml
ssh-bin: sshpass
# Please change 123456 to ${{secrets.SSH_PASSWD}}
args: |
  -p 123456
  ssh
host: "192.168.50.10"
run: |
  printf "Hello\n"
```

#### args

The parameters passed to `ssh_bin`, for example `-q -o ServerAliveInterval=60`

#### Main Stage

#### run

- required: true
- Type: String

The command executed on the remote host, the shell called during execution depends on the default login shell on the remote host.

#### allow-run-failure

```yaml
with:
  allow-run-failure: true
```

- Type: `bool`
- Default: `true`

- When it's true, if run errors out, it will not cause the current step to crash.
- When it's false, if run errors out, then this step will exit abnormally.

### Post Stage

#### post-run

Similar to run, but it runs in the Post stage.

#### allow-post-run-failure

Similar to allow-run-failure, but it captures the exit status of post-run, not run.

## Outputs

| Outputs               |
| --------------------- |
| pre-local-cmd-success |
| main-run-success      |
| post-run-success      |

You can use `${{steps."ssh-step-id".outputs.main-run-success}}`, and change "ssh-step-id" to a specific id, to judge whether run is successful.

```rs
Success => true,
Failure | Not-Run => false
```

Please see the example below:

```yaml
      - name: ssh-action
        id: act
        uses: 2moe/local-ssh-action@v0
        with:
          log-level: debug
          args: -q
          host: rv64
          pre-local-workdir: /tmp
          pre-local-cmd: pwd
          pre-local-cmd-async: false
          allow-pre-local-cmd-failure: false
          pre-sleep: 1
          pre-timeout: 20
          run: printf "It's on the remote-host\n"
          allow-run-failure: false
          post-run: exit 127
          allow-post-run-failure: true

      - name: get ssh-action outputs
        run: |
          printf "
            pre-local: ${{steps.act.outputs.pre-local-cmd-success}}
            main: ${{steps.act.outputs.main-run-success}}
            post: ${{steps.act.outputs.post-run-success}}
            "
```

The output result is:

```log
21:59:05.171 [DEBUG] ssh_action_wasm:140 set_pre_local_workdir()
21:59:05.193 [INFO] ssh_action_wasm:153 old local working directory: /var/runners/2moe-private/_work/private/private
21:59:05.196 [INFO] ssh_action_wasm:155 new local working directory: /tmp
21:59:05.199 [DEBUG] ssh_action_wasm:102 run_pre_local_cmd()
21:59:05.200 [DEBUG] ssh_action_wasm:114 raw: pwd
21:59:05.201 [DEBUG] ssh_action_wasm:87 split_shell_cmd()
21:59:05.204 [INFO] ssh_action_wasm:96 cmd: pwd, args: []
21:59:05.206 [INFO] ssh_action_wasm:127 running pwd...
/tmp
21:59:05.241 [DEBUG] ssh_action_wasm:230 pre_timeout_connection()
21:59:05.244 [DEBUG] ssh_action_wasm:244 InputConfig { ssh_bin: "ssh", args: Some(["-q"]), host: "rv64", pre_exit_cmd: ["exit"], pre_sleep: 1, pre_timeout: 20, run: "printf \"It's on the remote-host\\n\"", allow_run_failure: false, post_run: Some("exit 127"), allow_post_run_failure: true }
21:59:05.248 [INFO] ssh_action_wasm:247 sleep: 1s
21:59:06.249 [DEBUG] ssh_action_wasm:263 pre_args: ["-q", "rv64", "exit"]
21:59:06.251 [INFO] ssh_action_wasm:264 pre_timeout: 20s
21:59:09.268 [DEBUG] ssh_action_wasm:279 main_args: ["-q", "rv64", "printf \"It's on the remote-host\\n\""]
It's on the remote-host
21:59:09.952 [DEBUG] ssh_action_wasm:294 post_args: ["-q", "rv64", "exit 127"]
21:59:10.624 [ERROR] ssh_action_wasm:298 Post-Run: JsValue(Error: Failed to run the task:
    cmd: ssh
    args: -q,rv64,exit 127
    exit-code: 127
    sync: true
...
```

```yaml
  pre-local: true
  main: true
  post: false
```
