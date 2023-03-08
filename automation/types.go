package automation

import (
	"context"
)

type AutomationStep interface {
	Do(ctx context.Context, bytes []byte) (context.Context, []byte, error)
	GetName() string
}

type AutomationsMap map[string]*Automation
