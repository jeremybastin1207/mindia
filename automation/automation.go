package automation

import (
	"context"
)

type Automation struct {
	Steps []AutomationStep
	Name  string
}

type AutomationArgs struct {
	Steps []AutomationStep
	Name  string
}

func NewAutomation(args *AutomationArgs) *Automation {
	return &Automation{
		Steps: args.Steps,
		Name:  args.Name,
	}
}

func (a *Automation) Run(name string, bytes []byte) (context.Context, []byte, error) {
	ctx := context.Background()
	ctx = context.WithValue(ctx, NamerCtxKey{}, name)

	for _, step := range a.Steps {
		ctx2, bytes2, err := step.Do(ctx, bytes)
		if err != nil {
			return ctx, nil, err
		}
		ctx = ctx2
		bytes = bytes2
	}

	return ctx, bytes, nil
}

func (a *Automation) GetName() string {
	return a.Name
}

func ToMap(arr []*Automation) AutomationsMap {
	mp := AutomationsMap{}
	for _, el := range arr {
		mp[el.GetName()] = el
	}
	return mp
}
