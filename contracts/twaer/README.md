# TWAER contract

The TWAER (Time-Weighted Average Exchange Rate) contract calculates and maintains time-weighted average exchange rates for maxBTC tokens by querying AUM (Assets Under Management) from oracle contracts and token supply from the bank module. It stores historical exchange rate data points and provides both instant and time-weighted average exchange rates, with configurable time windows and automatic cleanup of expired data.
