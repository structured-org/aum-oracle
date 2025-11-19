export default interface Multisig {
    multisigAddress: string;
    /*
        payload is the arbitrary data we want to submit on the destination chain
        returns the transaction hash
    */
    submitProposal(payload: Buffer, timestamp: number): Promise<string>;
    /*
        Safe id is a transaction hash (string)
        SQUADs id is a proposal number
        returns transaction hash
    */
    executeProposal(id: any): Promise<string>;
    /*
        Safe returns its own ListResponse of pending proposals
        SQUADs is supposed to return the 'List<T>'
    */
    getPendingProposals(): Promise<any>;
}