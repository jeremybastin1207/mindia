package automation

import "context"

type JpegConverterConfig struct {
	*AutomationStepConfig `yaml:",inline"`
}

type JpegConverter struct {
	*JpegConverterConfig `yaml:",inline"`
}

func NewJpegConverter(config *JpegConverterConfig) *JpegConverter {
	return &JpegConverter{
		JpegConverterConfig: config,
	}
}

func (c *JpegConverter) GetChildren() []*Automation {
	return c.Children
}

func (c *JpegConverter) Do(ctx context.Context) (context.Context, error) {
	return ctx, nil
}
