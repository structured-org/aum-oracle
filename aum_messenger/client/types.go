package client

// NextRound contains receiver contract's next consensus round information.
type NextRound struct {
	// Round is the next consensus round number.
	Round uint64 `json:"round"`
	// Timestamp is the timestamp of the next consensus round beginning.
	Timestamp uint64 `json:"timestamp"`
}
