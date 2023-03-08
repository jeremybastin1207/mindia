package transformer

import (
	"bufio"
	"bytes"
	"context"
	"image"
	"image/jpeg"
	"mindia/types"

	"golang.org/x/image/draw"
)

type Resizer struct {
}

func NewResizer() *Resizer {
	return &Resizer{}
}

func (r *Resizer) Transform(ctx context.Context, bytes2 []byte) ([]byte, error) {
	size := ctx.Value("size").(types.Size)

	input := bytes.NewReader(bytes2)
	decodedInput, _ := jpeg.Decode(input)

	width := decodedInput.Bounds().Max.X
	height := decodedInput.Bounds().Max.Y

	if width < height {
		height = int(size.Height)
		width = int(float32(size.Height) / float32((height)) * float32(width))
	} else {
		height = int(float32(size.Width) / float32((width)) * float32(height))
		width = int(size.Width)
	}

	dst := image.NewRGBA(image.Rect(0, 0, width, height))

	draw.NearestNeighbor.Scale(dst, dst.Rect, decodedInput, decodedInput.Bounds(), draw.Over, nil)

	buff := new(bytes.Buffer)
	w2 := bufio.NewWriter(buff)
	jpeg.Encode(w2, dst, &jpeg.Options{Quality: jpeg.DefaultQuality})

	return buff.Bytes(), nil
}

func (r *Resizer) GetName() string {
	return "resizer"
}
