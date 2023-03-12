package automation

import "context"

type RetrieveFunc func(Name string) (Body, error)

type SourceConfig struct {
	*AutomationStepConfig
	Load RetrieveFunc
}

func NewSource(config *SourceConfig) *Source {
	return &Source{
		AutomationStep: AutomationStep{
			Children: config.AutomationStepConfig.Children,
		},
	}
}

type Source struct {
	AutomationStep
	*SourceConfig
}

func (s *Source) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)

	Body, err := s.Load(actx.Name)
	if err != nil {
		return ctx, err
	}
	actx.Body = Body
	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)

	return ctx, nil
}
