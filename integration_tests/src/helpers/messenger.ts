// TypeScript types and helpers for interacting with the mock controller

// =============================================================================
// MOCK CONTROLLER
// =============================================================================

export class MockController {
  private readonly baseUrl: string;

  constructor(port: number) {
    this.baseUrl = `http://localhost:${port}`;
  }

  // Generic helper for making POST requests
  private async postToMockController<T>(
    endpoint: string,
    data: T,
  ): Promise<void> {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(
        `Failed to call ${endpoint}: ${response.status} ${response.statusText} - ${errorText}`,
      );
    }
  }

  // Generic helper for making GET requests
  private async getFromMockController<T>(
    endpoint: string,
    queryParams?: Record<string, string>,
  ): Promise<T> {
    let url = `${this.baseUrl}${endpoint}`;

    if (queryParams) {
      const searchParams = new URLSearchParams(queryParams);
      url += `?${searchParams.toString()}`;
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(
        `Failed to call ${endpoint}: ${response.status} ${response.statusText} - ${errorText}`,
      );
    }

    return response.json();
  }

  /**
   * Set mock Binance USD-margined perpetual futures positions
   */
  async setBinanceUmPositions(positions: BinanceUMPosition[]): Promise<void> {
    await this.postToMockController('/mock/binance/umpositions', positions);
  }

  /**
   * Get current mock Binance USD-margined perpetual futures positions
   */
  async getBinanceUmPositions(): Promise<BinanceUMPosition[]> {
    return this.getFromMockController<BinanceUMPosition[]>(
      '/mock/binance/umpositions',
    );
  }

  /**
   * Set mock Binance Portfolio Margin account info
   */
  async setBinancePMAccountInfo(account: BinancePmAccountInfo): Promise<void> {
    await this.postToMockController('/mock/binance/pmaccountinfo', account);
  }

  /**
   * Get current mock Binance Portfolio Margin account info
   */
  async getBinancePMAccountInfo(): Promise<BinancePmAccountInfo> {
    return this.getFromMockController<BinancePmAccountInfo>(
      '/mock/binance/pmaccountinfo',
    );
  }

  /**
   * Set mock Binance Portfolio Margin account balance
   */
  async setBinancePMAccountBalance(
    balances: BinancePmAccountBalance[],
  ): Promise<void> {
    await this.postToMockController('/mock/binance/pmaccountbalance', balances);
  }

  /**
   * Get current mock Binance Portfolio Margin account balance
   */
  async getBinancePMAccountBalance(): Promise<BinancePmAccountBalance[]> {
    return this.getFromMockController<BinancePmAccountBalance[]>(
      '/mock/binance/pmaccountbalance',
    );
  }

  /**
   * Set mock Binance spot account info
   */
  async setBinanceSpotAccountInfo(
    account: BinanceSpotAccountInfo,
  ): Promise<void> {
    await this.postToMockController('/mock/binance/spotaccountinfo', account);
  }

  /**
   * Get current mock Binance spot account info
   */
  async getBinanceSpotAccountInfo(): Promise<BinanceSpotAccountInfo> {
    return this.getFromMockController<BinanceSpotAccountInfo>(
      '/mock/binance/spotaccountinfo',
    );
  }

  /**
   * Set mock Jupiter perps custody info
   */
  async setJupiterPerpsCustodyInfo(
    publicKey: string,
    data: JupiterPerpsCustodyAccount,
  ): Promise<void> {
    const request: JupiterPerpsCustodyInfoRequest = {
      publicKey,
      data,
    };
    await this.postToMockController('/mock/jupiter/custodyinfo', request);
  }

  /**
   * Get current mock Jupiter perps custody info
   * @param custodyPublicKey - The custody account public key (base58 string)
   */
  async getJupiterPerpsCustodyInfo(
    custodyPublicKey: string,
  ): Promise<JupiterPerpsCustodyAccount> {
    return this.getFromMockController<JupiterPerpsCustodyAccount>(
      '/mock/jupiter/custodyinfo',
      {
        custody: custodyPublicKey,
      },
    );
  }

  /**
   * Set mock Jupiter pool info
   */
  async setJupiterPoolInfo(poolInfo: JupiterPoolAccount): Promise<void> {
    await this.postToMockController('/mock/jupiter/poolinfo', poolInfo);
  }

  /**
   * Get current mock Jupiter pool info
   */
  async getJupiterPoolInfo(): Promise<JupiterPoolAccount> {
    return this.getFromMockController<JupiterPoolAccount>(
      '/mock/jupiter/poolinfo',
    );
  }

  /**
   * Set mock Solana token mint
   */
  async setSolanaTokenMint(tokenMint: SolanaTokenMint): Promise<void> {
    await this.postToMockController('/mock/solana/tokenmint', tokenMint);
  }

  /**
   * Get current mock Solana token mint
   */
  async getSolanaTokenMint(): Promise<SolanaTokenMint> {
    return this.getFromMockController<SolanaTokenMint>(
      '/mock/solana/tokenmint',
    );
  }

  /**
   * Set mock Solana token account balance
   */
  async setSolanaTokenAccountBalance(
    publicKey: string,
    balance: SolanaUiTokenAmount,
  ): Promise<void> {
    const request: SolanaTokenAccountBalanceRequest = {
      publicKey,
      data: balance,
    };
    await this.postToMockController(
      '/mock/solana/tokenaccountbalance',
      request,
    );
  }

  /**
   * Get current mock Solana token account balance
   * @param tokenPublicKey - The token account public key (base58 string)
   */
  async getSolanaTokenAccountBalance(
    tokenPublicKey: string,
  ): Promise<SolanaUiTokenAmount> {
    return this.getFromMockController<SolanaUiTokenAmount>(
      '/mock/solana/tokenaccountbalance',
      {
        token: tokenPublicKey,
      },
    );
  }

  /**
   * Set mock Solana native balance
   */
  async setSolanaNativeBalance(balance: SolanaUiTokenAmount): Promise<void> {
    await this.postToMockController('/mock/solana/nativebalance', balance);
  }

  /**
   * Get current mock Solana native balance
   */
  async getSolanaNativeBalance(): Promise<SolanaUiTokenAmount> {
    return this.getFromMockController<SolanaUiTokenAmount>(
      '/mock/solana/nativebalance',
    );
  }
  /**
   * Enable timeouts for binance queries
   */
  async enableBinanceTimeout(): Promise<void> {
    await this.postToMockController('/mock/binance/enabletimeout', {});
  }

  /**
   * Disable timeouts for binance queries
   */
  async disableBinanceTimeout(): Promise<void> {
    await this.postToMockController('/mock/binance/disabletimeout', {});
  }

  /**
   * Enable timeouts for solana queries
   */
  async enableSolanaTimeout(): Promise<void> {
    await this.postToMockController('/mock/solana/enabletimeout', {});
  }

  /**
   * Disable timeouts for solana queries
   */
  async disableSolanaTimeout(): Promise<void> {
    await this.postToMockController('/mock/solana/disabletimeout', {});
  }
}

// =============================================================================
// BINANCE TYPES
// =============================================================================

// Binance UMPosition (USD-margined perpetual futures position)
export interface BinanceUMPosition {
  symbol: string;
  positionAmt: string;
  entryPrice: string;
  markPrice: string;
  unrealizedProfit: string;
  liquidationPrice: string;
  leverage: string;
  maxNotional: string;
  maxNotionalValue: string;
  positionSide: string;
  initialMargin: string;
  maintMargin: string;
  positionInitialMargin: string;
  openOrderInitialMargin: string;
  notional: string;
  bidNotional: string;
  askNotional: string;
  updateTime: number;
}

// Binance Portfolio Margin Account Info
export interface BinancePmAccountInfo {
  uniMMR: string;
  accountEquity: string;
  actualEquity: string;
  accountInitialMargin: string;
  accountMaintMargin: string;
  accountStatus: string;
  virtualMaxWithdrawAmount: string;
  totalAvailableBalance: string;
  totalMarginOpenLoss: string;
  updateTime: number;
}

// Binance Portfolio Margin Balance
export interface BinancePmAccountBalance {
  asset: string;
  totalWalletBalance: string;
  crossMarginAsset: string;
  crossMarginBorrowed: string;
  crossMarginFree: string;
  crossMarginInterest: string;
  crossMarginLocked: string;
  umWalletBalance: string;
  umUnrealizedPNL: string;
  cmWalletBalance: string;
  cmUnrealizedPNL: string;
  updateTime: number;
  negativeBalance: string;
}

// Binance Spot Account Info
export interface BinanceSpotAccountInfo {
  makerCommission: number;
  takerCommission: number;
  buyerCommission: number;
  sellerCommission: number;
  commissionRates: BinanceCommissionRates;
  canTrade: boolean;
  canWithdraw: boolean;
  canDeposit: boolean;
  updateTime: number;
  accountType: string;
  balances: BinanceSpotBalance[];
  permissions: string[];
  uid: number;
}

// Binance Spot Account Balance
export interface BinanceSpotBalance {
  asset: string;
  free: string;
  locked: string;
}

// Binance Spot Account Commission Rates
export interface BinanceCommissionRates {
  maker: string;
  taker: string;
  buyer: string;
  seller: string;
}

// =============================================================================
// JUPITER TYPES
// =============================================================================

// Jupiter Perps Custody Info
export interface JupiterPerpsCustodyInfo {
  publicKey: string;
  data: JupiterPerpsCustodyAccount;
}

// Jupiter Perps Custody Account
export interface JupiterPerpsCustodyAccount {
  discriminator: number[]; // [8]byte as array of numbers
  pool: string; // PublicKey as base58 string
  mint: string; // PublicKey as base58 string
  tokenAccount: string; // PublicKey as base58 string
  decimals: number;
  isStable: boolean;
  oracle: JupiterPerpsCustodyOracle;
  pricing: JupiterPerpsCustodyPricing;
  permissions: JupiterPerpsCustodyPermissions;
  targetRatioBps: number;
  assets: JupiterPerpsCustodyAssets;
  fundingRateState: JupiterPerpsCustodyFundingRateState;
  bump: number;
  tokenAccountBump: number;
  increasePositionBps: number;
  decreasePositionBps: number;
  maxPositionSizeUsd: number;
  dovesOracle: string; // PublicKey as base58 string
  jumpRateState: JupiterPerpsCustodyJumpRateState;
  dovesAgOracle: string; // PublicKey as base58 string
  priceImpactBuffer: JupiterPerpsCustodyPriceImpactBuffer;
}

// Jupiter Perps Custody Oracle
export interface JupiterPerpsCustodyOracle {
  oracleAccount: string; // PublicKey as base58 string
  oracleType: number;
  buffer: number;
  maxPriceAgeSec: number;
}

// Jupiter Perps Custody Pricing
export interface JupiterPerpsCustodyPricing {
  tradeImpactFeeScalar: number;
  buffer: number;
  swapSpread: number;
  maxLeverage: number;
  maxGlobalLongSizes: number;
  maxGlobalShortSizes: number;
}

// Jupiter Perps Custody Permissions
export interface JupiterPerpsCustodyPermissions {
  allowSwap: boolean;
  allowAddLiquidity: boolean;
  allowRemoveLiquidity: boolean;
  allowIncreasePosition: boolean;
  allowDecreasePosition: boolean;
  allowCollateralWithdrawal: boolean;
  allowLiquidatePosition: boolean;
}

// Jupiter Perps Custody Assets
export interface JupiterPerpsCustodyAssets {
  feesReserves: number;
  owned: number;
  locked: number;
  guaranteedUsd: number;
  globalShortSizes: number;
  globalShortAveragePrices: number;
}

// Jupiter Perps Custody Funding Rate State
export interface JupiterPerpsCustodyFundingRateState {
  cumulativeInterestRate: string; // Uint128 as string
  lastUpdate: number;
  hourlyFundingDbps: number;
}

// Jupiter Perps Custody Jump Rate State
export interface JupiterPerpsCustodyJumpRateState {
  minRateBps: number;
  maxRateBps: number;
  targetRateBps: number;
  targetUtilizationRate: number;
}

// Jupiter Perps Custody Price Impact Buffer
export interface JupiterPerpsCustodyPriceImpactBuffer {
  openInterest: string[]; // [60]int64 as array of strings
  lastUpdated: number;
  feeFactor: number;
  exponent: number;
  deltaImbalanceThreshold: number;
  maxFeeBps: number;
}

// Jupiter Pool Account
export interface JupiterPoolAccount {
  discriminator: number[]; // [8]byte as array of numbers
  name: string;
  custodies: string[]; // []PublicKey as array of base58 strings
  aumUsd: string; // Uint128 as string
  limit: JupiterPoolLimit;
  fees: JupiterPoolFees;
  poolApr: JupiterPoolApr;
  maxRequestExecutionSec: number;
  bump: number;
  lpTokenBump: number;
  inceptionTime: number;
  parameterUpdateOracle: Secp256k1Pubkey;
}

// Jupiter Pool Limit
export interface JupiterPoolLimit {
  maxAumUsd: string; // Uint128 as string
  tokenWeightageBufferBps: string; // Uint128 as string
  buffer: number;
}

// Jupiter Pool Fees
export interface JupiterPoolFees {
  swapMultiplier: number;
  stableSwapMultiplier: number;
  addRemoveLiquidityBps: number;
  swapBps: number;
  taxBps: number;
  stableSwapBps: number;
  stableSwapTaxBps: number;
  liquidationRewardBps: number;
  protocolShareBps: number;
}

// Jupiter Pool APR
export interface JupiterPoolApr {
  lastUpdated: number;
  feeAprBps: number;
  realizedFeeUsd: number;
}

// Secp256k1 Public Key
export interface Secp256k1Pubkey {
  prefix: number;
  key: number[]; // [32]uint8 as array of numbers
}

// Jupiter Perps Custody Info Request (for endpoint)
export interface JupiterPerpsCustodyInfoRequest {
  publicKey: string; // PublicKey as base58 string
  data: JupiterPerpsCustodyAccount;
}

// =============================================================================
// SOLANA TYPES
// =============================================================================

// Solana UiTokenAmount
export interface SolanaUiTokenAmount {
  amount: string;
  decimals: number;
}

// Solana Token Account Balance Request (for endpoint)
export interface SolanaTokenAccountBalanceRequest {
  publicKey: string; // Token's PublicKey as base58 string
  data: SolanaUiTokenAmount;
}

// Solana Token Mint
export interface SolanaTokenMint {
  Supply: number; // capitalized because json tag isn't defined in respective go struct
  Decimals: number; // capitalized because json tag isn't defined in respective go struct
}
