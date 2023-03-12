package automation

import "context"

type RetrieveFunc func(Name string) (Body, error)

type SourceConfig struct {
	Load RetrieveFunc
}

type Source struct {
	*SourceConfig
}

func NewSource(config *SourceConfig) *Source {
	return &Source{}
}

func (s *Source) GetChildren() []*Automation {
	return []*Automation{}
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
