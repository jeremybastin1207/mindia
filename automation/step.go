package automation

import (
	"context"
)

type AutomationDoer interface {
	Do(ctx context.Context) (context.Context, error)
	GetChildren() []*Automation
}

type AutomationStepConfig struct {
	Children []*Automation `yaml:"children"`
}

func (s *AutomationStep) GetChildren() []*Automation {
	return s.Children
}

type AutomationStep struct {
	Children []*Automation
}
