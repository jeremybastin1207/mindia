package automation

import "context"

type WatermarkerConfig struct {
	*AutomationStepConfig `yaml:",inline"`
}

func NewWatermarker(config *WatermarkerConfig) *Watermarker {
	return &Watermarker{
		AutomationStep:    *NewAutomationStep(config.AutomationStepConfig),
		WatermarkerConfig: config,
	}
}

type Watermarker struct {
	AutomationStep
	*WatermarkerConfig `yaml:",inline"`
}

func (n *Watermarker) Do(ctx context.Context) (context.Context, error) {
	return ctx, nil
}
