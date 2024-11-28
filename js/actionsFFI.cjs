/**
 * Retrieves the value of an input parameter from the environment variables.
 * The input name is converted to uppercase and spaces are replaced with underscores
 * to match the expected environment variable format.
 *
 * ```
 * name -> env.INPUT_{NAME}
 * e.g., a-b-c -> INPUT_A-B-C -> env.INPUT_A-B-C
 * Note: It is INPUT_A-B-C, not INPUT_A_B_C.
 * ```
 *
 * @param {string} name - The name of the input parameter.
 * @returns {string} - If the value of the environment variable is empty, return an empty string; otherwise, return its corresponding value(trimmed).
 */
function getInput(name) {
    const formattedEnv = `INPUT_${name.replace(/ /g, "_").toUpperCase()}`;
    const value = process.env[formattedEnv];
    if (!value) {
        // throw new Error(`Input required and not supplied: ${formattedEnv}`);
        return "";
    }
    return value.trim();
}

/**
 * Sets the value of an output parameter by appending it to the GitHub Actions output file.(env.GITHUB_OUTPUT)
 * The output key and value are written in the format `key=value`.
 *
 * @param {string} key - The name of the output parameter.
 * @param {string} value - The value of the output parameter.
 */
function setOutput(key, value) {
    const fs = require("node:fs");
    const outputPath = process.env.GITHUB_OUTPUT || "";
    const lineBreak = require('os').EOL;

    if (outputPath) {
        return fs.appendFileSync(outputPath, `${key}=${value}${lineBreak}`);
    }
    console.log(`::set-output name=${key}::${value}`);
}


/**
 * Logs all environment variables for debugging purposes.
 *
 * @returns {boolean} - Always returns true.
 */
function showAllEnvs() {
    console.trace(process.env);
    return true;
}

module.exports = { getInput, setOutput, showAllEnvs };
