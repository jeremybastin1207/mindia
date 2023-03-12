package automation

import (
	"context"
)

type Body []byte

type AutomationCtxKey struct{}

type AutomationCtx struct {
	Name string
	Body Body
}

type UniqueNamer struct{}

type AutomationStepConfig struct {
	Namer    UniqueNamer
	Children []*Automation
}

type NamerStep interface {
	NamerFunc(Name string) string
}

type AutomationStep interface {
	GetChildren() []*Automation
	Do(ctx context.Context) (context.Context, error)
}

type AutomationConfig struct {
	ApplyToCurrentFiles bool
	Namer               NamerStep
	Steps               []AutomationStep
}

type Automation struct {
	*AutomationConfig
}

func NewAutomation(config *AutomationConfig) *Automation {
	return &Automation{
		AutomationConfig: config,
	}
}

func (a *Automation) Run(actx AutomationCtx, namerFunc UniqueNamerFunc, source *Source, sinker *Sinker) error {
	var err error
	ctx := context.Background()
	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)

	namer := NewNamer(&NamerConfig{
		AutomationStepConfig: &AutomationStepConfig{
			Children: []*Automation{},
		},
		NamerFunc: namerFunc,
	})

	steps := a.Steps
	if source != nil {
		steps = append([]AutomationStep{source}, steps...)
	}
	steps = append([]AutomationStep{namer}, steps...)
	steps = append(steps, sinker)

	for _, step := range steps {
		ctx, err = step.Do(ctx)
		if err != nil {
			return err
		}
		actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
		for _, sa := range step.GetChildren() {
			sa.Run(actx, sa.AutomationConfig.Namer.NamerFunc, nil, sinker)
		}
	}
	return nil
}
