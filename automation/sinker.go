package automation

import "context"

type SinkerConfig struct {
	*AutomationStepConfig
	Sink func(AutomationCtx)
}

func NewSinker(config *SinkerConfig) *Sinker {
	return &Sinker{
		AutomationStep: *NewAutomationStep(config.AutomationStepConfig),
		SinkerConfig:   config,
	}
}

type Sinker struct {
	AutomationStep
	*SinkerConfig
}

func (s *Sinker) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
	if actx.Body == nil {
		return ctx, nil
	}
	s.Sink(actx)
	return ctx, nil
}
