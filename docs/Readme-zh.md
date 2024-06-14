# Local-SSH-Action

| Languages/語言                            | ID         |
| ----------------------------------------- | ---------- |
| 中文                                      | zh-Hans-CN |
| [English](../Readme.md)                   | en-Latn-US |
| [中文 (Traditional)](./Readme-zh-Hant.md) | zh-Hant-TW |

与其他的 ssh actions 不同，此 action 依赖于本地的 ssh。

在 debian/ubuntu 上，如果没有 ssh 客户端的话，那得要手动安装。

```sh
apt install openssh-client
```

## 创建此 action 的初衷是什么？

初衷是为了方便在 github actions 中连接虚拟机。

因为有些平台的交叉编译工具链有点难搞，就算搞定了，那搭建测试环境可能也没有您想象中的简单。

比如我想要在 Linux arm64 上构建 OpenBSD riscv64 的 rust bin crate (二进制软件包)，那么用常规方法会很辛苦。
而直接开一个虚拟机就方便多了，不需要理解很多细节，就能直接上手了。

最初，它只是几行简单的 javascript 脚本。
后来，我根据在 github actions 上使用 ssh 的经验，用 rust 重写了核心逻辑，添加了不少实用的选项。

相信它定能给您带来良好 ssh action 的体验。

## Inputs

| 输入                        | 描述                                                                             | 默认值    |
| --------------------------- | -------------------------------------------------------------------------------- | --------- |
| log-level                   | 可选值："trace", "debug", "info", "warn", "error", "off"                         | info      |
| pre-local-workdir           | 本地工作目录                                                                     |           |
| pre-local-cmd               | 在连接到 ssh 之前，通过 NodeJS 的 `spawn()` 或 `spawnSync()` 执行命令            |           |
| pre-local-cmd-async         | 类型：boolean。当为 true 时，异步运行 `pre-local-cmd`                            | true      |
| allow-pre-local-cmd-failure | 类型：boolean。当为 true 时，允许 `pre-local-cmd` 失败 (忽略 pre-local-cmd 出错) | false     |
| pre-sleep                   | 在连接到 ssh 之前，阻塞特定时间，单位为秒                                        | 0         |
| pre-timeout                 | 由于 ssh 连接可能会失败，指定 `pre-timeout` 可以让其不断重试连接，直到超时       | 0         |
| pre-exit-cmd                | 测试 ssh 所需的命令                                                              | exit      |
| host                        | 远程服务器的主机名或 IP                                                          | 127.0.0.1 |
| ssh-bin                     | 在不稳定的网络环境中，您可能需要使用特定的ssh，而不是 openssh 客户端             | ssh       |
| run                         | 在远程主机上执行的命令                                                           |           |
| allow-run-failure           | 类型：boolean。当为 true 时，允许 `run` 失败                                     | false     |
| post-run                    | 在 `run` 完成后，您可以继续运行 `post-run`                                       |           |
| allow-post-run-failure      | 类型：boolean                                                                    | true      |
| args                        | SSH 客户端参数，e.g., `-q`                                                       |           |

## Get Started

先来看一个简单的例子

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
          pre-local-cmd: printf "pre-local-cmd 是在本地主机上运行的\n"
          pre-timeout: 20
          args: -q
          host: android-mobile
          run: |
            printf "这是在远程主机上哦\n"
            /system/bin/toybox uname -m
          allow-run-failure: true
          post-run: printf "Bye\n"
```

您可能无法理解上面的内容，并且这个 workflow 可能会失败。

别着急，容我慢慢与您道来。

上面这段例子本质上是在执行 `ssh android-mobild uname -m`，如果运行 actions 的机器上不存在 `android-mobile` 的 ssh 配置，那就连接不上。

解决方法很简单，创建一个配置就行了。

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

配置只需要创建一次，如果自托管的机器上已经创建了 android-mobile 的配置，那么下次连接前，就无需再创建了。

如果不想创建配置，也不想用 ed25519 密钥，想要在单个 step 中用密码连接，那也是可以做到的。
不过在此之前，我们得要先了解其基本用法。

## 详细说明

with 可以接很多选项，比如：

```yaml
with:
  log-level: debug
```

本节将对这些选项进行详细说明。

### Core

在展开说明之前，您得要先了解一个核心要素：流程是分阶段进行的。

分别是:

- Pre
- Main
- Post

ssh 连接的正式阶段为 Main, 连接之前为 Pre，连接之后为 Post。

Q: 为什么要分阶段呢？

A:

- 因为在连接 ssh 前，可能会失败。故在 Pre 阶段，您可以在指定时间内，不断尝试连接，直到连接成功。
- 连接完 ssh 后，您可能需要执行一些清理任务 (e.g., 关闭虚拟机)
  - 清理任务可能会失败，将 main 与 post 分开，可以分别处理任务状态。
    - i.e., 不允许 main 失败，但允许 post 失败。

### Pre 阶段

#### pre-local-workdir

```yaml
with:
  pre-local-workdir: /path/to/local-dir
```

- 类型: String（文件路径）

这个选项修改的是本地的工作目录，而不是远程 ssh 的目录。

Q: 为什么有这个选项呢？

A: 假设 ssh 的配置文件不在 **~/.ssh**，而是位于特定的目录，通过指定连接 ssh 前的目录，可以简化一些操作。

#### pre-local-cmd

```yaml
with:
  pre-local-cmd: ls -lah
```

- 类型: String

在连接 ssh 前，通过 NodeJS 的 `spawn()` 或 `spawnSync()` 来执行命令。

> 这是在本地主机上运行的，而不是远程主机。

假设 `pre-local-cmd: ls -la -h` 且没有配置 `pre-local-cmd-async`, 那么它会自动解析为 `spawn("ls", ["-la", "-h"])`

#### pre-local-cmd-async

```yaml
with:
  pre-local-cmd-async: true
```

- 类型: `bool`
- 默认: `true`

- 当为 true 时，以异步方式运行命令。
  - 也就是说，在连接远程 ssh 之前，让本机任务在后台运行。
- 当为 false 时，同步（阻塞）运行命令。
  - 在连接远程 ssh 之前，必须等待 pre-local-cmd 任务完成，才能继续连接 ssh。

默认为 true, i.e., 默认是异步的。

#### allow-pre-local-cmd-failure

```yaml
with:
  allow-pre-local-cmd-failure: true
```

- 类型: `bool`
- 默认: `true`

- 当为 true 时，忽略 pre-local-cmd 的错误。
  - 更准确的说法是：当 pre-local-cmd 失败时，不会导致当前 step 崩溃。
- 当为 false 时，若 pre-local-cmd 出错，则此 step 将异常退出。

#### pre-sleep

```yaml
with:
  pre-sleep: 0
```

- typescript 类型: number
- rust       类型: u32
- 默认: 0

在连接 ssh 前，同步（阻塞）特定的时间，单位为秒。

假设您要连接一个正在处于重启中的机器，那现在连接的话，可能过几秒就断开了。

此时，需要强制阻塞，等待几秒，让它彻底关机，再尝试连接。

- 例子：
  - `pre-sleep: 1`, 阻塞 1 秒。
  - `pre-sleep: 30`, 阻塞 30 秒。

之所以强调 rust 类型，是因为在内部实现中，是通过以下函数来解析的。

```rust
fn parse_gh_num(input: &str) -> u32 {
    match get_action_input(input).trim() {
        "" => 0,
        x => x.parse().unwrap_or(0),
    }
}
```

u32 必须要 `>=0`, i.e., 您不能使用 `pre_sleep: -1` 来表示无限阻塞。

P.S. 如果需要在指定时间内测试能否正常连接，那请使用 `pre-timeout`，而不是 `pre-sleep`。

#### pre-timeout

```yaml
with:
  pre-timeout: 0
```

- 类型: u32
- 默认: 0

由于 ssh 连接可能会失败，指定 pre-timeout 可以让您在特定的时间内进行等待。

与阻塞的 pre-sleep 不同，对于 pre-timeout，一旦测试连接成功，就会退出等待。

- 例子：
  - `pre-timeout: 120`, 等待的超时时间为 120 秒。

假设您要连接一个正在处于开机中的虚拟机，那么要等它连接到网络，并启动完 `sshd` 进程后，才能连接上去。

若 `pre-timeout: 30`，而虚拟机的开机时间 + 启动 sshd 的时间为 10秒，那么它不会干等 30 秒，在 10+ 秒时， 一旦测试连接成功，就会退出等待。

#### pre-exit-cmd

```yaml
with:
  pre-exit-cmd: exit
```

- 类型: String
- 默认: "exit"

测试 ssh 连接所需的命令，默认为 exit。

只有当 `pre-timeout > 0` 时，此选项才会生效。

假设 `pre-timeout: 20`, `pre-exit-cmd: exit`，`host: netbsd-vm`

那么它会在 20 秒内，不断进行 `ssh netbsd-vm exit` 的连接测试。

只有当测试连接成功后，才会继续下一步。

如果在 20 秒后失败，那么整个 step 都会失败。

### 多阶段共用

#### log-level

```yaml
with:
  log-level: debug
```

- 类型: `enum LogLevel`
- 默认: `info`
- 可选值: "trace", "debug", "info", "warn", "error", "off"

其中 trace 最详细，debug 第二详细，off 无日志。

#### host

```yaml
with:
  host: "127.0.0.1"
```

- 类型：String
- 默认: "127.0.0.1"

远程主机名称或 IP 地址。

#### ssh-bin

```yaml
with:
  ssh-bin: ssh
```

- 类型: String
- 默认: "ssh"

在不稳定的网络环境下，您可能需要使用特定的 ssh，而不是 openssh 客户端。

只要命令语法符合 `{ssh-bin} {args} {host} {run}` 这条规则，那用什么都可以。

假设有 `adb -s android-14 shell [run]`，此时您可以使用

```yaml
ssh-bin: adb
args: -s android-14
host: shell
run: |
  ls -lh
  toybox printf "I am Android\n"
```

假设您需要自动输入密码，那么 `sshpass -p $passwd ssh 192.168.50.10 [run]` 可以转换为：

```yaml
ssh-bin: sshpass
# 请将 123456 改成 ${{secrets.SSH_PASSWD}}
args: |
  -p 123456
  ssh
host: "192.168.50.10"
run: |
  printf "Hello\n"
```

#### args

传给 `ssh_bin` 的参数，例如 `-q -o ServerAliveInterval=60`

#### Main 阶段

#### run

- required: true
- 类型: 字符串

在远程主机上执行的命令，执行时所调用的 shell 取决于远程主机上的默认 login shell。

#### allow-run-failure

```yaml
with:
  allow-run-failure: true
```

- 类型: `bool`
- 默认: `true`

- 当为 true 时，若 run 出错，不会导致当前 step 崩溃。
- 当为 false 时，若 run 出错，则此 step 将异常退出。

### Post 阶段

#### post-run

类似于 run, 但是在 Post 阶段运行的。

#### allow-post-run-failure

类似于 allow-run-failure， 但捕获的是 post-run 退出状态，而不是 run。

## Outputs

| 输出                  |
| --------------------- |
| pre-local-cmd-success |
| main-run-success      |
| post-run-success      |

您可以用 `${{steps."ssh-step-id".outputs.main-run-success}}` 来判断 run 是否成功。

> 请将 "ssh-step-id" 修改为特定 id

```rs
成功 => true,
失败 | 未运行 => false
```

请看下面这个例子：

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

输出的结果是：

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
