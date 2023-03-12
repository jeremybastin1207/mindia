package automation

type Body []byte

type AutomationCtxKey struct{}

type AutomationCtx struct {
	Name    string
	Body    Body
	Outputs []string
}
