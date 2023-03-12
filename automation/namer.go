package automation

import (
	"context"
)

type UniqueNamerFunc func(Name string) string

type NamerConfig struct {
	*AutomationStepConfig `yaml:",inline"`
	NamerFunc             UniqueNamerFunc
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

	if n.NamerFunc != nil {
		actx.Name = n.NamerFunc(actx.Name)
		ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)
	}

	return ctx, nil
}
