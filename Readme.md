# Local-SSH-Action V0

Unlike other ssh actions, this one depends on local ssh.

It is more suitable for **self-hosted** runners.

On Ubuntu, if you don't have an ssh client, you will need to install `openssh-client` manually.

## Usage

```yaml
- uses: 2moe/local-ssh-action@v0
  with:
    #   description: Remote Server Host
    #   required: false
    #   default: 127.0.0.1
    host: ''

    # description: Commands to be executed on remote server
    # required: true
    run: ''

    # description: SSH Client args, one for each line.
    # required: false
    args: ''
```

## Example

```yaml
      - name: create ssh config
        run: |
          echo '
          Host riscv64-sbc
            Hostname 192.168.123.234
            User user
            Port 8222
            IdentityFile ~/.ssh/key/rv64_ed25519
            StrictHostKeyChecking no
          ' > cfg
          install -Dm600 cfg ~/.ssh/config.d/rv64.sshconf
          printf "%s\n" 'Include config.d/*.sshconf' >> ~/.ssh/config

      - uses: 2moe/local-ssh-action@v0
        with:
          host: riscv64-sbc
          run: |
            ls -a
            sleep 2
            cat /etc/os-release
            sleep 5
            echo OK
          args: |
            # Only one arg can be entered in a line.

            # quiet operation
            -q

            # extra options
            -o
            ServerAliveInterval=60

            # ...
```
