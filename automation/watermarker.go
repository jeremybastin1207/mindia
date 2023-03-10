package automation

import "context"

type WatermarkerConfig struct {
	*AutomationStepConfig `yaml:",inline"`
}

type Watermarker struct {
	*WatermarkerConfig `yaml:",inline"`
}

func NewWatermarker(config *WatermarkerConfig) *Watermarker {
	return &Watermarker{
		WatermarkerConfig: config,
	}
}

func (w *Watermarker) GetChildren() []*Automation {
	return w.Children
}

func (n *Watermarker) Do(ctx context.Context) (context.Context, error) {
	return ctx, nil
}
