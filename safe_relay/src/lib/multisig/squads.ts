import type Multisig from "./index";

export type SquadsMultisigConfig = {}

export default class SquadsMultisig implements Multisig {
    multisigAddress: string;

    constructor(multisigAddress: string) {
        this.multisigAddress = multisigAddress;
    }

    async submitProposal(): Promise<string | null> {
        return ""
    }

    async executeProposal(id: number): Promise<string> {
        return ""
    }

    async getPendingProposals(): Promise<any> {
        return []
    }
}