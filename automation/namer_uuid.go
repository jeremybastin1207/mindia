package automation

import (
	"context"
	"path/filepath"

	"github.com/google/uuid"
)

type NamerUuidConfig struct {
	*AutomationStepConfig `yaml:",inline"`
}

type NamerUuid struct {
	*NamerUuidConfig `yaml:",inline"`
}

func NewNamerUuid(config *NamerUuidConfig) *NamerUuid {
	return &NamerUuid{
		NamerUuidConfig: config,
	}
}

func (n *NamerUuid) GetChildren() []*Automation {
	return n.Children
}

func (n *NamerUuid) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
	actx.Name = uuid.New().String() + filepath.Ext(actx.Name)
	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)
	return ctx, nil
}
