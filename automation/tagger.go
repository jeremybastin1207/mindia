package automation

import "context"

type TaggerConfig struct {
	*AutomationStepConfig `yaml:",inline"`
}

type Tagger struct {
	*TaggerConfig `yaml:",inline"`
}

func NewTagger(config *TaggerConfig) *Tagger {
	return &Tagger{
		TaggerConfig: config,
	}
}

func (t *Tagger) GetChildren() []*Automation {
	return t.Children
}

func (t *Tagger) Do(ctx context.Context) (context.Context, error) {
	return ctx, nil
}
