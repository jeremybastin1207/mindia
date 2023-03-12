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

func NewAutomationStep(config *AutomationStepConfig) *AutomationStep {
	children := []*Automation{}
	if config.Children != nil {
		children = config.Children
	}
	a := AutomationStep{
		Children: children,
	}
	return &a
}

type AutomationStep struct {
	Children []*Automation
}
