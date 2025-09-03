package mocks

//go:generate mockgen -source=./../../messenger/binance/interface.go -destination ./binance-messenger/mocks.go
//go:generate mockgen -source=./../../messenger/jupiter/interface.go -destination ./jupiter-messenger/mocks.go
//go:generate mockgen -source=./../../messenger/arbitrary_data/interface.go -destination ./arbitrarydata-messenger/mocks.go
