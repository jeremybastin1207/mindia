package types

import (
	"fmt"
	"path/filepath"

	"github.com/google/uuid"
)

type Size struct {
	Width  int32 `json:"width"`
	Height int32 `json:"height"`
}

type File struct {
	Dir           string `json:"dir"`
	Name          string `json:"name"`
	ContentType   string `json:"content_type"`
	ContentLength int64  `json:"content_length"`
}

type Image struct {
	File
	Size
}

type FileInput struct {
	Dir           string
	Name          string
	ContentType   string
	ContentLength int64
}

func NewFile(in *FileInput) (*File, error) {
	return &File{
		Dir:           in.Dir,
		Name:          in.Name,
		ContentType:   in.ContentType,
		ContentLength: in.ContentLength,
	}, nil
}

type ImageInput struct {
	Dir           string
	Name          string
	ContentType   string
	ContentLength int64
	Width         int32
	Height        int32
}

func NewImage(in *ImageInput) (*Image, error) {
	return &Image{
		File: File{
			Dir:           in.Dir,
			Name:          in.Name,
			ContentType:   in.ContentType,
			ContentLength: in.ContentLength,
		},
		Size: Size{
			Width:  in.Width,
			Height: in.Height,
		},
	}, nil
}

func GenerateName(filename string) string {
	return uuid.New().String() + filepath.Ext(filename)
}

func (f *File) ToString() string {
	return fmt.Sprintf(
		"dir %s, file %s, content-type: %s, content-length: %d",
		f.Dir,
		f.Name,
		f.ContentType,
		f.ContentLength,
	)
}

func GetDir(path string) string {
	dir, _ := filepath.Split(path)
	return dir
}

func GetName(path string) string {
	_, name := filepath.Split(path)
	return name
}
