package automation

import (
	"bufio"
	"bytes"
	"context"
	"image"
	"image/jpeg"
	"mindia/types"

	"golang.org/x/image/draw"
)

type ResizerConfig struct {
	*AutomationStepConfig `yaml:",inline"`
	Size                  types.Size `yaml:"size"`
}

type Resizer struct {
	*ResizerConfig `yaml:",inline"`
}

func NewResizer(config *ResizerConfig) *Resizer {
	return &Resizer{
		ResizerConfig: config,
	}
}

func (r *Resizer) GetChildren() []*Automation {
	return r.Children
}

func (r *Resizer) Do(ctx context.Context) (context.Context, error) {
	actx := ctx.Value(AutomationCtxKey{}).(AutomationCtx)
	input := bytes.NewReader(actx.Body)
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
	actx.Body = buff.Bytes()

	ctx = context.WithValue(ctx, AutomationCtxKey{}, actx)
	return ctx, nil
}
