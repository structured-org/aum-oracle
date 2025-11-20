export default interface Multisig {
    multisigAddress: string;

    /*
        payload is the arbitrary data we want to submit on the destination chain
        Safe returns null
        SQUADS returns the transaction hash
    */
    submitProposal(payload: Buffer, timestamp: number): Promise<string | null>;

    /*
        Safe id is a transaction hash (string)
        SQUADS id is a proposal number
        returns transaction hash
    */
    executeProposal(id: any): Promise<string>;

    /*
        Safe returns its own ListResponse of pending proposals
        SQUADS is supposed to return the 'List<T>'
    */
    getPendingProposals(): Promise<any>;
}