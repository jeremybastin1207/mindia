package automation

import (
	"context"
	"mindia/automation/namer"
)

type AutomationConfig struct {
	Namer namer.Namer
	Steps []AutomationDoer
}

func NewAutomation(config *AutomationConfig) *Automation {
	return &Automation{
		AutomationConfig: config,
	}
}

type Automation struct {
	*AutomationConfig
}

func (a *Automation) Run(actx AutomationCtx, namer namer.Namer, source *Source, sinker *Sinker) error {
	var err error
	ctx := context.Background()
	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)

	steps := a.Steps
	if source != nil {
		steps = append([]AutomationDoer{source}, steps...)
	}
	steps = append([]AutomationDoer{
		NewNamer(&NamerConfig{
			AutomationStepConfig: &AutomationStepConfig{
				Children: []*Automation{},
			},
			Namer: namer,
		}),
	}, steps...)
	steps = append(steps, sinker)

	for _, step := range steps {
		ctx, err = step.Do(ctx)
		if err != nil {
			return err
		}
		for _, child := range step.GetChildren() {
			actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
			child.Run(actx, child.AutomationConfig.Namer, nil, sinker)
		}
	}
	return nil
}

func (a *Automation) DryRun(actx AutomationCtx, namer namer.Namer) (context.Context, error) {
	ctx := context.Background()
	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)

	steps := a.Steps
	steps = append([]AutomationDoer{
		NewNamer(&NamerConfig{
			AutomationStepConfig: &AutomationStepConfig{
				Children: []*Automation{},
			},
			Namer: namer,
		}),
	}, steps...)

	var err error
	for _, step := range steps {
		ctx, err = step.Do(ctx)
		if err != nil {
			return nil, err
		}
		for _, child := range step.GetChildren() {
			actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
			ctx, err = child.DryRun(actx, child.Namer)
			if err != nil {
				return nil, err
			}
		}
	}
	return ctx, nil
}
