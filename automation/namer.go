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
		AutomationStep: AutomationStep{
			Children: config.AutomationStepConfig.Children,
		},
		NamerConfig: config,
	}
}

type Namer struct {
	AutomationStep
	*NamerConfig `yaml:",inline"`
}

func (n *Namer) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)

	if n.Namer != nil {
		actx.Name = n.Namer.Name(actx.Name)
		ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)
	}

	return ctx, nil
}
