# Local-SSH-Action

| Languages/語言                            | ID         |
| ----------------------------------------- | ---------- |
| 中文 (Traditional)                       | zh-Hant-TW |
| [English](../Readme.md)                   | en-Latn-US |
| [中文 (Simplified)](./Readme-zh.md)      | zh-Hans-CN |

與其他的 ssh actions 不同，此 action 依賴於本地的 ssh。

在 debian/ubuntu 上，如果沒有 ssh 客戶端的話，那得要手動安裝。

```sh
apt install openssh-client
```

## 建立此 action 的初衷是什麼？

初衷是為了方便在 github actions 中連線虛擬機器。

因為有些平臺的交叉編譯工具鏈有點難搞，就算搞定了，那搭建測試環境可能也沒有您想象中的簡單。

比如我想要在 Linux arm64 上構建 OpenBSD riscv64 的 rust bin crate (二進位制軟體包)，那麼用常規方法會很辛苦。
而直接開一個虛擬機器就方便多了，不需要理解很多細節，就能直接上手了。

最初，它只是幾行簡單的 javascript 指令碼。
後來，我根據在 github actions 上使用 ssh 的經驗，用 rust 重寫了核心邏輯，添加了不少實用的選項。

相信它定能給您帶來良好 ssh action 的體驗。

## Inputs

| 輸入                        | 描述                                                                             | 預設值    |
| --------------------------- | -------------------------------------------------------------------------------- | --------- |
| log-level                   | 可選值："trace", "debug", "info", "warn", "error", "off"                         | info      |
| pre-local-workdir           | 本地工作目錄                                                                     |           |
| pre-local-cmd               | 在連線到 ssh 之前，透過 NodeJS 的 `spawn()` 或 `spawnSync()` 執行命令            |           |
| pre-local-cmd-async         | 型別：boolean。當為 true 時，非同步執行 `pre-local-cmd`                            | true      |
| allow-pre-local-cmd-failure | 型別：boolean。當為 true 時，允許 `pre-local-cmd` 失敗 (忽略 pre-local-cmd 出錯) | false     |
| pre-sleep                   | 在連線到 ssh 之前，阻塞特定時間，單位為秒                                        | 0         |
| pre-timeout                 | 由於 ssh 連線可能會失敗，指定 `pre-timeout` 可以讓其不斷重試連線，直到超時       | 0         |
| pre-exit-cmd                | 測試 ssh 所需的命令                                                              | exit      |
| host                        | 遠端伺服器的主機名或 IP                                                          | 127.0.0.1 |
| ssh-bin                     | 在不穩定的網路環境中，您可能需要使用特定的ssh，而不是 openssh 客戶端             | ssh       |
| run                         | 在遠端主機上執行的命令                                                           |           |
| allow-run-failure           | 型別：boolean。當為 true 時，允許 `run` 失敗                                     | false     |
| post-run                    | 在 `run` 完成後，您可以繼續執行 `post-run`                                       |           |
| allow-post-run-failure      | 型別：boolean                                                                    | true      |
| args                        | SSH 客戶端引數，e.g., `-q`                                                       |           |

## Get Started

先來看一個簡單的例子

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
          pre-local-cmd: printf "pre-local-cmd 是在本地主機上執行的\n"
          pre-timeout: 20
          args: -q
          host: android-mobile
          run: |
            printf "這是在遠端主機上哦\n"
            /system/bin/toybox uname -m
          allow-run-failure: true
          post-run: printf "Bye\n"
```

您可能無法理解上面的內容，並且這個 workflow 可能會失敗。

彆著急，容我慢慢與您道來。

上面這段例子本質上是在執行 `ssh android-mobild uname -m`，如果執行 actions 的機器上不存在 `android-mobile` 的 ssh 配置，那就連線不上。

解決方法很簡單，建立一個配置就行了。

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

配置只需要建立一次，如果自託管的機器上已經建立了 android-mobile 的配置，那麼下次連線前，就無需再建立了。

如果不想建立配置，也不想用 ed25519 金鑰，想要在單個 step 中用密碼連線，那也是可以做到的。
不過在此之前，我們得要先了解其基本用法。

## 詳細說明

with 可以接很多選項，比如：

```yaml
with:
  log-level: debug
```

本節將對這些選項進行詳細說明。

### Core

在展開說明之前，您得要先了解一個核心要素：流程是分階段進行的。

分別是:

- Pre
- Main
- Post

ssh 連線的正式階段為 Main, 連線之前為 Pre，連線之後為 Post。

Q: 為什麼要分階段呢？

A:

- 因為在連線 ssh 前，可能會失敗。故在 Pre 階段，您可以在指定時間內，不斷嘗試連線，直到連線成功。
- 連線完 ssh 後，您可能需要執行一些清理任務 (e.g., 關閉虛擬機器)
  - 清理任務可能會失敗，將 main 與 post 分開，可以分別處理任務狀態。
    - i.e., 不允許 main 失敗，但允許 post 失敗。

### Pre 階段

#### pre-local-workdir

```yaml
with:
  pre-local-workdir: /path/to/local-dir
```

- 型別: String（檔案路徑）

這個選項修改的是本地的工作目錄，而不是遠端 ssh 的目錄。

Q: 為什麼有這個選項呢？

A: 假設 ssh 的配置檔案不在 **~/.ssh**，而是位於特定的目錄，透過指定連線 ssh 前的目錄，可以簡化一些操作。

#### pre-local-cmd

```yaml
with:
  pre-local-cmd: ls -lah
```

- 型別: String

在連線 ssh 前，透過 NodeJS 的 `spawn()` 或 `spawnSync()` 來執行命令。

> 這是在本地主機上執行的，而不是遠端主機。

假設 `pre-local-cmd: ls -la -h` 且沒有配置 `pre-local-cmd-async`, 那麼它會自動解析為 `spawn("ls", ["-la", "-h"])`

#### pre-local-cmd-async

```yaml
with:
  pre-local-cmd-async: true
```

- 型別: `bool`
- 預設: `true`

- 當為 true 時，以非同步方式執行命令。
  - 也就是說，在連線遠端 ssh 之前，讓本機任務在後臺執行。
- 當為 false 時，同步（阻塞）執行命令。
  - 在連線遠端 ssh 之前，必須等待 pre-local-cmd 任務完成，才能繼續連線 ssh。

預設為 true, i.e., 預設是非同步的。

#### allow-pre-local-cmd-failure

```yaml
with:
  allow-pre-local-cmd-failure: true
```

- 型別: `bool`
- 預設: `true`

- 當為 true 時，忽略 pre-local-cmd 的錯誤。
  - 更準確的說法是：當 pre-local-cmd 失敗時，不會導致當前 step 崩潰。
- 當為 false 時，若 pre-local-cmd 出錯，則此 step 將異常退出。

#### pre-sleep

```yaml
with:
  pre-sleep: 0
```

- typescript 型別: number
- rust       型別: u32
- 預設: 0

在連線 ssh 前，同步（阻塞）特定的時間，單位為秒。

假設您要連線一個正在處於重啟中的機器，那現在連線的話，可能過幾秒就斷開了。

此時，需要強制阻塞，等待幾秒，讓它徹底關機，再嘗試連線。

- 例子：
  - `pre-sleep: 1`, 阻塞 1 秒。
  - `pre-sleep: 30`, 阻塞 30 秒。

之所以強調 rust 型別，是因為在內部實現中，是透過以下函式來解析的。

```rust
fn parse_gh_num(input: &str) -> u32 {
    match get_action_input(input).trim() {
        "" => 0,
        x => x.parse().unwrap_or(0),
    }
}
```

u32 必須要 `>=0`, i.e., 您不能使用 `pre_sleep: -1` 來表示無限阻塞。

P.S. 如果需要在指定時間內測試能否正常連線，那請使用 `pre-timeout`，而不是 `pre-sleep`。

#### pre-timeout

```yaml
with:
  pre-timeout: 0
```

- 型別: u32
- 預設: 0

由於 ssh 連線可能會失敗，指定 pre-timeout 可以讓您在特定的時間內進行等待。

與阻塞的 pre-sleep 不同，對於 pre-timeout，一旦測試連線成功，就會退出等待。

- 例子：
  - `pre-timeout: 120`, 等待的超時時間為 120 秒。

假設您要連線一個正在處於開機中的虛擬機器，那麼要等它連線到網路，並啟動完 `sshd` 程序後，才能連線上去。

若 `pre-timeout: 30`，而虛擬機器的開機時間 + 啟動 sshd 的時間為 10秒，那麼它不會幹等 30 秒，在 10+ 秒時， 一旦測試連線成功，就會退出等待。

#### pre-exit-cmd

```yaml
with:
  pre-exit-cmd: exit
```

- 型別: String
- 預設: "exit"

測試 ssh 連線所需的命令，預設為 exit。

只有當 `pre-timeout > 0` 時，此選項才會生效。

假設 `pre-timeout: 20`, `pre-exit-cmd: exit`，`host: netbsd-vm`

那麼它會在 20 秒內，不斷進行 `ssh netbsd-vm exit` 的連線測試。

只有當測試連線成功後，才會繼續下一步。

如果在 20 秒後失敗，那麼整個 step 都會失敗。

### 多階段共用

#### log-level

```yaml
with:
  log-level: debug
```

- 型別: `enum LogLevel`
- 預設: `info`
- 可選值: "trace", "debug", "info", "warn", "error", "off"

其中 trace 最詳細，debug 第二詳細，off 無日誌。

#### host

```yaml
with:
  host: "127.0.0.1"
```

- 型別：String
- 預設: "127.0.0.1"

遠端主機名稱或 IP 地址。

#### ssh-bin

```yaml
with:
  ssh-bin: ssh
```

- 型別: String
- 預設: "ssh"

在不穩定的網路環境下，您可能需要使用特定的 ssh，而不是 openssh 客戶端。

只要命令語法符合 `{ssh-bin} {args} {host} {run}` 這條規則，那用什麼都可以。

假設有 `adb -s android-14 shell [run]`，此時您可以使用

```yaml
ssh-bin: adb
args: -s android-14
host: shell
run: |
  ls -lh
  toybox printf "I am Android\n"
```

假設您需要自動輸入密碼，那麼 `sshpass -p $passwd ssh 192.168.50.10 [run]` 可以轉換為：

```yaml
ssh-bin: sshpass
# 請將 123456 改成 ${{secrets.SSH_PASSWD}}
args: |
  -p 123456
  ssh
host: "192.168.50.10"
run: |
  printf "Hello\n"
```

#### args

傳給 `ssh_bin` 的引數，例如 `-q -o ServerAliveInterval=60`

### Main 階段

#### run

- required: true
- 型別: 字串

在遠端主機上執行的命令，執行時所呼叫的 shell 取決於遠端主機上的預設 login shell。

#### allow-run-failure

```yaml
with:
  allow-run-failure: true
```

- 型別: `bool`
- 預設: `true`

- 當為 true 時，若 run 出錯，不會導致當前 step 崩潰。
- 當為 false 時，若 run 出錯，則此 step 將異常退出。

### Post 階段

#### post-run

類似於 run, 但是在 Post 階段執行的。

#### allow-post-run-failure

類似於 allow-run-failure， 但捕獲的是 post-run 退出狀態，而不是 run。

## Outputs

| 輸出                  |
| --------------------- |
| pre-local-cmd-success |
| main-run-success      |
| post-run-success      |

您可以用 `${{steps."ssh-step-id".outputs.main-run-success}}` 來判斷 run 是否成功。

> 請將 "ssh-step-id" 修改為特定 id

```rs
成功 => true,
失敗 | 未執行 => false
```

請看下面這個例子：

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

輸出的結果是：

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
