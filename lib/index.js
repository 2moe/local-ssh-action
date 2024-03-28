const core = require("@actions/core");
const { spawn } = require("child_process");

const get = (input) => core.getInput(input);

const host = get("host") || "127.0.0.1";
const sshArgs = get("args");
const cmd = get("run");

const args = sshArgs
    .split(/[\r\n]+/)
    .map((x) => x.trim())
    .filter((x) => !x.match(/^\s|^#|^$/));

args.push(host);
args.push(cmd);

const ssh = spawn("ssh", args);

ssh.stdout.on("data", (data) => {
    process.stdout.write(data);
});

ssh.stderr.on("data", (data) => {
    process.stderr.write(data);
});

ssh.on("close", (code) => {
    if (code !== 0) {
        throw Error(`⚠️ Exit: ${code}`);
    }
    process.exit(code);
});
