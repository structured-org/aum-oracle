package oracle

// NextRound contains AUM contract's next consensus round information.
type NextRound struct {
	// Round is the next consensus round number.
	Round int64
	// Timestamp is the timestamp of the next consensus round beginning.
	Timestamp int64
}
