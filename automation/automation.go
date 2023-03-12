package automation

import (
	"context"
	"mindia/automation/namer"
)

type AutomationConfig struct {
	Namer namer.Namer
	Steps []AutomationDoer
}

type Automation struct {
	*AutomationConfig
}

func NewAutomation(config *AutomationConfig) *Automation {
	return &Automation{
		AutomationConfig: config,
	}
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
		actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
		for _, sa := range step.GetChildren() {
			sa.Run(actx, sa.AutomationConfig.Namer, nil, sinker)
		}
	}
	return nil
}
