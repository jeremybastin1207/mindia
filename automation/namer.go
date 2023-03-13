package automation

import (
	"context"
	"mindia/automation/namer"
)

type NamerConfig struct {
	*AutomationStepConfig `yaml:",inline"`
	Namer                 namer.Namer
}

func NewNamer(config *NamerConfig) *Namer {
	return &Namer{
		AutomationStep: *NewAutomationStep(config.AutomationStepConfig),
		NamerConfig:    config,
	}
}

type Namer struct {
	AutomationStep
	*NamerConfig `yaml:",inline"`
}

func (n *Namer) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
	name := actx.Name

	if n.Namer != nil {
		name = n.Namer.Name(name)
	}

	actx.Name = name
	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)

	return ctx, nil
}
