package transformer

import (
	"context"
)

type Transformer interface {
	Transform(ctx context.Context, bytes []byte) ([]byte, error)
	GetName() string
}
