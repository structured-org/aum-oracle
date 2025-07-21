import { sleep } from './sleep';
import { StargateClient } from '@cosmjs/stargate';
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';

export const waitFor = async (
  fn: () => Promise<boolean>,
  timeout: number = 10000,
  interval: number = 600,
): Promise<void> => {
  const start = Date.now();
  // eslint-disable-next-line no-constant-condition
  while (true) {
    if (await fn()) {
      break;
    }
    if (Date.now() - start > timeout) {
      throw new Error('Timeout waiting for condition');
    }
    await sleep(interval);
  }
};


export const waitBlocks = async (
  blocks: number,
  client: StargateClient | CosmWasmClient,
  timeout = 120000,
): Promise<void> => {
  const start = Date.now();
  // const client = await StargateClient.connect(this.rpc);
  const initBlock = await client.getBlock();
  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      const block = await client.getBlock();
      if (block.header.height - initBlock.header.height >= blocks) {
        break;
      }
      if (Date.now() - start > timeout) {
        throw new Error('Timeout waiting for the specific block');
      }
    } catch (e) {
      //noop
    }
    await sleep(1000);
  }
};
