package automation

import (
	"bufio"
	"bytes"
	"context"
	"image"
	"image/jpeg"
	"mindia/types"

	"github.com/rs/zerolog/log"
	"golang.org/x/image/draw"
)

type ResizerConfig struct {
	Size types.Size `yaml:"size"`
}

type Resizer struct {
	*ResizerConfig `yaml:",inline"`
}

func NewResizer(config *ResizerConfig) *Resizer {
	return &Resizer{
		ResizerConfig: config,
	}
}

type ResizerCtxKey struct{}

func (r *Resizer) Do(ctx context.Context, bytes2 []byte) (context.Context, []byte, error) {
	log.Info().Msg("running resizer")

	input := bytes.NewReader(bytes2)
	decodedInput, _ := jpeg.Decode(input)

	width := decodedInput.Bounds().Max.X
	height := decodedInput.Bounds().Max.Y

	if width < height {
		height = int(r.Size.Height)
		width = int(float32(r.Size.Height) / float32((height)) * float32(width))
	} else {
		height = int(float32(r.Size.Width) / float32((width)) * float32(height))
		width = int(r.Size.Width)
	}

	dst := image.NewRGBA(image.Rect(0, 0, width, height))

	draw.NearestNeighbor.Scale(dst, dst.Rect, decodedInput, decodedInput.Bounds(), draw.Over, nil)

	buff := new(bytes.Buffer)
	w2 := bufio.NewWriter(buff)
	jpeg.Encode(w2, dst, &jpeg.Options{Quality: jpeg.DefaultQuality})

	ctx = context.WithValue(ctx, ResizerCtxKey{}, r.Size)

	return ctx, buff.Bytes(), nil
}

func (r *Resizer) GetName() string {
	return "resizer"
}
