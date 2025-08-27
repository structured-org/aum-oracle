package messenger

import "time"

// OperationalConfig is the configuration for a messenger's operational parameters.
type OperationalConfig struct {
	// FailureDelay is the delay taken when a messenger fails to fetch or submit data to
	// prevent the messenger from spamming.
	FailureDelay time.Duration `yaml:"failure_delay"`
	// PreSubmitDelay is an additional delay taken before submitting data to prevent the
	// messenger from submitting data too soon because of possible time desync.
	PreSubmitDelay time.Duration `yaml:"pre_submit_delay"`
	// FetchDataTimeout is a timeout for fetch data operation.
	FetchDataTimeout time.Duration `yaml:"fetch_data_timeout"`
}

// NextRound contains receiver's next consensus round information.
type NextRound struct {
	// Round is the next consensus round number.
	Round uint64
	// Timestamp is the timestamp of the next consensus round beginning.
	Timestamp uint64
}
