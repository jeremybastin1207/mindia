package automation

import "context"

type SinkerConfig struct {
	Sink func(AutomationCtx)
}

type Sinker struct {
	*SinkerConfig
}

func NewSinker(config *SinkerConfig) *Sinker {
	return &Sinker{
		SinkerConfig: config,
	}
}

func (s *Sinker) GetChildren() []*Automation {
	return []*Automation{}
}

func (s *Sinker) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
	s.Sink(actx)
	return ctx, nil
}

func (s *Sinker) IsSync(ctx context.Context) (context.Context, error) {
	return ctx, nil
}
