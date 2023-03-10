package automation

import (
	"context"
	"fmt"
	"path/filepath"
	"strings"
)

type NamerSuffixConfig struct {
	*AutomationStepConfig `yaml:",inline"`
	Suffix                string `yaml:"suffix"`
}

type NamerSuffix struct {
	*NamerSuffixConfig `yaml:",inline"`
}

func NewNamerSuffix(config *NamerSuffixConfig) *NamerSuffix {
	return &NamerSuffix{
		NamerSuffixConfig: config,
	}
}

func (n *NamerSuffix) GetChildren() []*Automation {
	return n.Children
}

func (n *NamerSuffix) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)

	extension := filepath.Ext(actx.Name)
	basename := strings.TrimSuffix(actx.Name, extension)
	actx.Name = fmt.Sprintf("%s_%s%s", basename, n.Suffix, extension)

	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)
	return ctx, nil
}
