import Manager from '../index';
import { Logger } from 'pino';
import { EthereumConfig, NeutronConfig } from '../../lib/config';
import SafeMultisig, { SafeMultisigConfig } from '../../lib/multisig/safe';
import Safe from '@safe-global/protocol-kit';
import { HDAccount, mnemonicToAccount } from 'viem/accounts';
import SafeApiKit from '@safe-global/api-kit';
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import {
  createPublicClient,
  decodeFunctionData,
  http,
  PublicClient,
} from 'viem';
import { mainnet } from 'viem/chains';

import RECEIVER_ABI from '../../generic/Receiver.abi.json';

export default class RelayEthereum implements Manager {
  private readonly logger: Logger;
  private ethereum: EthereumConfig;
  private neutron: NeutronConfig;

  private msParticipantIndex?: number;
  private cosmWasmClient?: CosmWasmClient;
  private ethClient?: PublicClient;
  private ethAccount?: HDAccount;
  private safeMultisig?: SafeMultisig;
  private safeApi?: SafeApiKit;

  constructor(
    logger: Logger,
    ethereumConfig: EthereumConfig,
    neutronConfig: NeutronConfig,
  ) {
    this.logger = logger;
    this.ethereum = ethereumConfig;
    this.neutron = neutronConfig;
  }

  async init(): Promise<void> {
    /* Neutron */
    this.cosmWasmClient = await CosmWasmClient.connect(this.neutron.rpc);
    /* Ethereum */
    const account = mnemonicToAccount(this.ethereum.mnemonic!);
    this.ethAccount = account;

    const pk = account.getHdKey().privateKey;
    this.safeApi = new SafeApiKit({
      chainId: 1n,
      apiKey: this.ethereum.safeApiKey,
    });
    const safeConfig: SafeMultisigConfig = {
      safeClient: await Safe.init({
        provider: this.ethereum.rpc,
        signer: `0x${Buffer.from(pk!).toString('hex')}`,
        safeAddress: this.ethereum.safeAddress,
      }),
      safeApi: this.safeApi,
      receiverAddress: this.ethereum.receiverAddress,
      signer: account,
    };
    this.safeMultisig = new SafeMultisig(
      this.ethereum.safeAddress,
      this.logger,
      safeConfig,
    );
    this.ethClient = createPublicClient({
      chain: mainnet,
      transport: http(this.ethereum.rpc),
    }) as PublicClient;
    const safeInfo = await this.safeApi?.getSafeInfo(this.safeMultisig?.multisigAddress!)!;
    this.msParticipantIndex = safeInfo.owners.findIndex(owner => owner.toLowerCase() === this.ethAccount?.address.toLowerCase());
  }

  async tick(): Promise<void> {
    const delayTimeMs = this.ethereum.proposalDelay * this.msParticipantIndex! * 1000
    await new Promise((resolve) => setTimeout(resolve, delayTimeMs));

    const neutronData = await this.getTWAERData();
    const ethereumData = await this.getReceverData();
    const pendingProposals = await this.safeMultisig?.getPendingProposals()!;
    if (pendingProposals.results.length > 0) {
      this.logger.info(
        'There are pending proposals. Skipping new proposal submission.',
      );
      const proposals = pendingProposals.results.filter(
        (p) =>
          p.to === this.ethereum.receiverAddress &&
          (p.confirmations || []).every(
            (c) =>
              c.owner.toLowerCase() !== this.ethAccount?.address.toLowerCase(),
          ),
      );
      if (proposals.length > 0) {
        this.logger.trace('There is a proposal to vote on.');
        const proposal = proposals[0];
        if (!proposal) return;
        if (this.ethereum.verifyValuesOnVote && proposal) {
          const dataFromProposal = decodeFunctionData({
            abi: RECEIVER_ABI,
            data: proposal.data as `0x${string}`,
          });
          this.logger.trace('Data from Proposal: %o', dataFromProposal);
          if (dataFromProposal.functionName !== 'publish') {
            this.logger.error(
              'Unexpected function name in proposal: %s',
              dataFromProposal.functionName,
            );
            return;
          }
          const [erFromProposal, tsFromProposal] = dataFromProposal.args as [
            BigInt,
            BigInt,
          ];
          if (
            erFromProposal !== neutronData.er ||
            Number(tsFromProposal) !== neutronData.ts
          ) {
            this.logger.warn(
              'Data in proposal does not match Neutron data. Neutron ER: %o, TS: %d; Proposal ER: %d, TS: %d',
              neutronData.er,
              neutronData.ts,
              Number(erFromProposal),
              Number(tsFromProposal),
            );
            return;
          }
          this.logger.info('Data in proposal matches Neutron data');
        }
        this.logger.info('Voting on proposal ID: %s', proposal.safeTxHash);
        await this.safeMultisig?.confirmProposal(proposal.safeTxHash);
      }
      const readyProposals = pendingProposals.results.filter(
        (p) =>
          p.to === this.ethereum.receiverAddress &&
          (p.confirmations || []).length === p.confirmationsRequired,
      );
      if (readyProposals.length > 0) {
        this.logger.info('There are proposals ready to be executed.');
        const readyProposal = readyProposals[0];
        if (!readyProposal) return;
        this.logger.info('Ready proposal ID: %s', readyProposal.safeTxHash);
        await this.safeMultisig?.executeProposal(readyProposal.safeTxHash);
      }
    } else {
      if (
        neutronData.ts > ethereumData.ts ||
        neutronData.er !== ethereumData.er
      ) {
        this.logger.info(
          'New data available. Neutron TS: %d, Ethereum TS: %d',
          neutronData.ts,
          ethereumData.ts,
        );
        await this.safeMultisig?.submitProposal(neutronData.er, neutronData.ts);
      }
    }
  }

  private async getTWAERData(): Promise<{ er: BigInt; ts: number }> {
    this.logger.info('Querying TWAER data from Neutron');
    const result = await this.cosmWasmClient?.queryContractSmart(
      this.neutron.twaerContract,
      {
        get_twaer: {},
      },
    );
    this.logger.trace('TWAER Query Result: %o', result);
    return {
      er: this.toBigIntTimes10Pow(result.twaer),
      ts: result.published_at,
    };
  }

  private async getReceverData(): Promise<{ er: BigInt; ts: number }> {
    this.logger.info('Querying Receiver data from Ethereum');
    const [er, ts] = (await this.ethClient?.readContract({
      address: this.ethereum.receiverAddress as `0x${string}`,
      abi: RECEIVER_ABI,
      functionName: 'getLatest',
      args: [],
      authorizationList: undefined,
    })) as [BigInt, BigInt];
    this.logger.trace('Receiver Query Result: %o', [er, ts]);
    return { er, ts: Number(ts) };
  }

  private toBigIntTimes10Pow(str: string, decimals = 18): BigInt {
    str = str.trim();
    if (str.length === 0) {
      return 0n;
    }

    const [intPart = '', fracPart = ''] = str.split('.');
    const combined = intPart + fracPart.padEnd(decimals, '0');
    const normalized = combined.slice(0, intPart.length + decimals);
    return BigInt(normalized);
  }
}
