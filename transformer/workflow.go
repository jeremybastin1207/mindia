package transformer

import (
	"context"
	"fmt"
)

type Workflow struct {
	steps []Transformer
}

func (w *Workflow) Run(bytes []byte) []byte {
	ctx := context.Background()

	for _, step := range w.steps {
		bytes2, err := step.Transform(ctx, bytes)
		if err != nil {
			fmt.Printf("Error. %v", err)
		}
		bytes = bytes2
	}

	return bytes
}
