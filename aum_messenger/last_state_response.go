package main

import (
	"time"

	msgr "github.com/structured-org/aum-messenger/messenger"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
)

func buildLastStateResponse(
	mode string,
	storeBinance *msgr.LastValueStore[*neutronclient.BinanceAumData],
	storeJupiter *msgr.LastValueStore[*neutronclient.JupiterAumData],
	now time.Time,
) lastStateResponse {
	resp := lastStateResponse{
		Mode: mode,
		Now:  now.UTC(),
		Networks: map[string]lastStateEntry{
			"binance": {},
			"jupiter": {},
		},
	}

	{
		v, ok, t := storeBinance.Load()
		if ok {
			tt := t
			resp.Networks["binance"] = lastStateEntry{Present: true, CapturedAt: &tt, Data: v}
		}
	}

	{
		v, ok, t := storeJupiter.Load()
		if ok {
			tt := t
			resp.Networks["jupiter"] = lastStateEntry{Present: true, CapturedAt: &tt, Data: v}
		}
	}

	return resp
}
