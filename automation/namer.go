package automation

import (
	"context"
	"fmt"
	"path/filepath"
	"strings"

	"github.com/rs/zerolog/log"
)

type Namer struct {
	Suffix string
}

type NamerArgs struct {
	Suffix string
}

func NewNamer(args *NamerArgs) *Namer {
	return &Namer{
		Suffix: args.Suffix,
	}
}

type NamerCtxKey struct{}

func (n *Namer) Do(ctx context.Context, b []byte) (context.Context, []byte, error) {
	log.Info().Msg("running namer")

	filename := ctx.Value(NamerCtxKey{}).(string)
	ctx = context.WithValue(ctx, NamerCtxKey{}, getName(filename, n.Suffix))
	return ctx, b, nil
}

func getName(filename, suffix string) string {
	extension := filepath.Ext(filename)
	basename := strings.TrimSuffix(filename, extension)
	return fmt.Sprintf("%s_%s%s", basename, suffix, extension)
}

func (n *Namer) GetName() string {
	return "namer"
}
