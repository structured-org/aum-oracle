import {Config} from "./lib/config";
import {getLogger} from "./lib/logger";

async function main() {
    const logger = getLogger();
    const config = new Config(logger);
    await config.init();
}

main();