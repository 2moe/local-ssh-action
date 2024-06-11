// import * as wasm from "../pkg/ssh_action_wasm.js";
const wasm = require("../pkg/ssh_action_wasm.js");

const _logger = wasm.init_logger();
// console.info(_logger.get_level());

wasm.set_pre_local_workdir();
wasm.run_pre_local_cmd();

const config = wasm.get_main_input_config();
wasm.ssh_connection(config);
