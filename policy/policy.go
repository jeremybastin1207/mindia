package policy

type PolicyConfig struct {
	ContentTypesAllowed []string `yaml:"content_types_allowed"`
	ContentLengthMax    int64    `yaml:"content_length_max"`
}

func NewPolicy(config *PolicyConfig) *Policy {
	return &Policy{
		PolicyConfig: config,
	}
}

type Policy struct {
	*PolicyConfig
}
