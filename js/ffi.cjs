const child = require("node:child_process");

/**
 * Spawns a process to run external command
 *
 * @param {string} cmd
 * @param {readonly string[]} args
 * @param {boolean} sync
 *
 * @throws {Error}
 */
function spawnCmd(cmd, args, sync) {
    const options = {
        stdio: "inherit",
    };

    const handleErr = () => {
        if (eCode === 0) {
            return;
        }
        const errMsg = `Failed to run the task:
    cmd: ${cmd}
    args: ${args}
    exit-code: ${eCode}
    sync: ${sync}
    `;
        throw Error(errMsg);
    };

    let eCode;

    // sync:
    if (sync) {
        eCode = child.spawnSync(cmd, args, options).status;
        return handleErr();
    }

    // async:
    child.spawn(cmd, args, options).on("close", (code) => {
        eCode = code;
        handleErr();
    });
}
module.exports.spawnCmd = spawnCmd;

/**
 * This is a synchronous thread blocking function that pauses for a specified number of milliseconds. When milliseconds are passed in, it will automatically multiply by 1000 to convert to seconds.
 *
 * @param {number} time
 */
function sleep(time) {
    const endTime = new Date(new Date().getTime() + time * 1000);
    while (endTime > new Date());
}

module.exports.sleep = sleep;

/**
 * Runs within the specified time and keep retrying.
 *
 * timeout: ms => ms * 1000 => seconds
 *
 * @param {string} cmd
 * @param {readonly string[] | undefined} args
 * @param {number} timeout
 * @returns {boolean}
 */
function retryCommandWithinTime(cmd, args, timeout) {
    const startTime = Date.now();

    while (Date.now() - startTime <= timeout * 1000) {
        let code = 2;
        try {
            code = child.spawnSync(cmd, args, {
                stdio: "inherit",
            }).status;
        } catch (error) {
            console.error(`Error executing "${cmd}"`, error);
        }

        if (code === 0) {
            return true;
        }
        sleep(1);
    }
    console.error(
        `Command "${cmd}" did not complete successfully within the specified timeout(${timeout}s).`,
    );
    return false;
}
module.exports.retryCommandWithinTime = retryCommandWithinTime;

/**
 * See also `process.exit()`
 *
 * @param {number | undefined} code
 *
 * @returns {never}
 */
function nodeExit(code) {
    require("node:process").exit(code);
}
module.exports.nodeExit = nodeExit;
