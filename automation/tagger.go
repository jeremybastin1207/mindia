package automation

import "context"

type TaggerConfig struct {
	*AutomationStepConfig `yaml:",inline"`
}

func NewTagger(config *TaggerConfig) *Tagger {
	return &Tagger{
		AutomationStep: *NewAutomationStep(config.AutomationStepConfig),
		TaggerConfig:   config,
	}
}

type Tagger struct {
	AutomationStep
	*TaggerConfig `yaml:",inline"`
}

func (t *Tagger) Do(ctx context.Context) (context.Context, error) {
	return ctx, nil
}
