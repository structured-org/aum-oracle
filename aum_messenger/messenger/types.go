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
