package automation

import (
	"context"
	"mindia/automation/namer"
)

type UniqueNamerFunc func(Name string) string

type NamerConfig struct {
	*AutomationStepConfig `yaml:",inline"`
	Namer                 namer.Namer
}

type Namer struct {
	*NamerConfig `yaml:",inline"`
}

func NewNamer(config *NamerConfig) *Namer {
	return &Namer{
		NamerConfig: config,
	}
}

func (n *Namer) GetChildren() []*Automation {
	return n.Children
}

func (n *Namer) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)

	if n.Namer != nil {
		actx.Name = n.Namer.Name(actx.Name)
		ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)
	}

	return ctx, nil
}
