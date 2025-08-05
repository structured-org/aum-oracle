package messenger

// NextRound contains receiver's next consensus round information.
type NextRound struct {
	// Round is the next consensus round number.
	Round uint64
	// Timestamp is the timestamp of the next consensus round beginning.
	Timestamp uint64
}
