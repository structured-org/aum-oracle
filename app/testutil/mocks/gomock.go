package mocks

//go:generate mockgen -source=./../../oracle/binance/interface.go -destination ./binance-oracle/mocks.go
//go:generate mockgen -source=./../../oracle/jupiter/interface.go -destination ./jupiter-oracle/mocks.go
