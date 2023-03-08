package automation

import (
	"context"
	"fmt"
	"path/filepath"
	"strings"

	"github.com/rs/zerolog/log"
)

type NamerConfig struct {
	Suffix string `yaml:"suffix"`
}

type Namer struct {
	*NamerConfig `yaml:",inline"`
}

func NewNamer(config *NamerConfig) *Namer {
	return &Namer{
		NamerConfig: config,
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
