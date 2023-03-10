package automation

import (
	"context"
)

type AutomationCtxKey struct{}

type AutomationCtx struct {
	Name string
	Body []byte
}

type AutomationStepConfig struct {
	Children []*Automation
}

type AutomationStep interface {
	GetChildren() []*Automation
	Do(ctx context.Context) (context.Context, error)
}

type AutomationConfig struct {
	Steps []AutomationStep
}

type Automation struct {
	*AutomationConfig
}

func NewAutomation(config *AutomationConfig) *Automation {
	return &Automation{
		AutomationConfig: config,
	}
}

func (a *Automation) Run(actx AutomationCtx, sinker Sinker) error {
	var err error
	ctx := context.Background()
	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)

	steps := a.Steps
	steps = append(steps, &sinker)

	for _, step := range steps {
		ctx, err = step.Do(ctx)
		if err != nil {
			return err
		}
		actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)

		for _, sa := range step.GetChildren() {
			sa.Run(actx, sinker)
		}

	}
	return nil
}
