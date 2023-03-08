package automation

import (
	"bufio"
	"bytes"
	"context"
	"image"
	"image/draw"
	"image/jpeg"
	"image/png"
	"os"

	"github.com/rs/zerolog/log"
)

type Watermarker struct {
}

func NewWatermarker() *Watermarker {
	return &Watermarker{}
}

func (w *Watermarker) Do(ctx context.Context, bytes2 []byte) (context.Context, []byte, error) {
	log.Info().Msg("running watermarker")

	watermarkPath := ctx.Value("watermarkPath").(string)

	input := bytes.NewReader(bytes2)
	decodedInput, _ := jpeg.Decode(input)

	watermark, _ := os.Open(watermarkPath)
	defer watermark.Close()
	decodedWatermark, _ := png.Decode(watermark)

	offset := image.Pt(0, 0)

	bounds := decodedInput.Bounds()
	img := image.NewRGBA(bounds)

	draw.Draw(img, bounds, decodedInput, image.ZP, draw.Src)
	draw.Draw(img, decodedWatermark.Bounds().Add(offset), decodedWatermark, image.ZP, draw.Over)

	buff := new(bytes.Buffer)
	w2 := bufio.NewWriter(buff)

	jpeg.Encode(w2, img, &jpeg.Options{Quality: jpeg.DefaultQuality})

	return ctx, buff.Bytes(), nil
}

func (w *Watermarker) GetName() string {
	return "watermarker"
}
