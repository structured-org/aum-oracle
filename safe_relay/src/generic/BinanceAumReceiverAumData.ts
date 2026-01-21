export const AUM_DATA = [{
    type: 'tuple',
    name: 'binanceData',
    components: [
        { name: 'unimmr', type: 'int256' },
        {
            name: 'positions',
            type: 'tuple[]',
            components: [
                { name: 'symbol', type: 'bytes32' },
                { name: 'amount', type: 'int256' },
                { name: 'pnl', type: 'int256' },
            ],
        },
        { name: 'umBalanceUsdt', type: 'int256' },
        {
            name: 'spotBalances',
            type: 'tuple[]',
            components: [
                { name: 'asset', type: 'bytes32' },
                { name: 'amount', type: 'int256' },
            ],
        },
        { name: 'pmAccountActualEquity', type: 'int256' },
        { name: 'withdrawableUsdt', type: 'int256' },
    ],
}];
