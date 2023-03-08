package transformer

import (
	"bufio"
	"bytes"
	"context"
	"image"
	"image/draw"
	"image/jpeg"
	"image/png"
	"os"
)

type Watermarker struct {
}

func NewWatermarker() *Watermarker {
	return &Watermarker{}
}

func (w *Watermarker) Transform(ctx context.Context, bytes2 []byte) ([]byte, error) {
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

	return buff.Bytes(), nil
}

func (w *Watermarker) GetName() string {
	return "watermarker"
}
