package automation

import (
	"context"
)

type JpegConverterConfig struct {
	*AutomationStepConfig `yaml:",inline"`
}

func NewJpegConverter(config *JpegConverterConfig) *JpegConverter {
	return &JpegConverter{
		AutomationStep:      *NewAutomationStep(config.AutomationStepConfig),
		JpegConverterConfig: config,
	}
}

type JpegConverter struct {
	AutomationStep
	*JpegConverterConfig `yaml:",inline"`
}

func (c *JpegConverter) Do(ctx context.Context) (context.Context, error) {
	return ctx, nil
}
