package arbitrary_data

type NeutronContractQuery struct {
	NeutronContractAddress string `yaml:"neutron_contract_address"`
	Query                  string `yaml:"query"`
	SolanaProgramID        string `yaml:"solana_program_id"`
	SolanaInstanceKey      string `yaml:"solana_instance_key"`
	OraclesListContract    string `yaml:"oracles_list_contract"`
}
