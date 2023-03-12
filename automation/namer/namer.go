package namer

type Namer interface {
	Name(filename string) string
}
