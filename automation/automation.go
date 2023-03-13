package automation

import (
	"context"
	"fmt"
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

func (a *Automation) DryRun(actx AutomationCtx) ([]string, error) {
	actx.Body = nil
	return a.Run(actx, nil, nil, nil)
}

func steps(steps []AutomationDoer, namer namer.Namer, source *Source, sinker *Sinker) []AutomationDoer {
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
	if sinker != nil {
		steps = append(steps, sinker)
	}
	return steps
}

func (a *Automation) Run(actx AutomationCtx, namer namer.Namer, source *Source, sinker *Sinker) ([]string, error) {
	var (
		outputs []string
		err     error
	)
	ctx := context.Background()
	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)

	for _, step := range steps(a.Steps, namer, source, sinker) {
		ctx, err = step.Do(ctx)
		if err != nil {
			fmt.Printf("Error: %s", err)
			return nil, err
		}

		actx = ctx.Value(AutomationCtxKey{}).(AutomationCtx)
		for _, child := range step.GetChildren() {
			out, err := child.Run(actx, child.AutomationConfig.Namer, nil, sinker)
			if err != nil {
				fmt.Printf("Error: %s", err)
				return nil, err
			}
			outputs = append(outputs, out...)
		}
	}

	actx = ctx.Value(AutomationCtxKey{}).(AutomationCtx)
	outputs = append(outputs, actx.Name)

	return outputs, nil
}
