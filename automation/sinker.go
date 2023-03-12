package automation

import "context"

type SinkerConfig struct {
	*AutomationStepConfig
	Sink func(AutomationCtx)
}

func NewSinker(config *SinkerConfig) *Sinker {
	return &Sinker{
		AutomationStep: AutomationStep{
			Children: config.AutomationStepConfig.Children,
		},
		SinkerConfig: config,
	}
}

type Sinker struct {
	AutomationStep
	*SinkerConfig
}

func (s *Sinker) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
	s.Sink(actx)
	return ctx, nil
}
