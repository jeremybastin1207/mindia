package storage

import (
	"mindia/types"
)

type StoragesMap map[string]Storage

type StorageConfig struct {
	StorageType string `yaml:"type"`
}

type Storage interface {
	Upload(in *UploadInput) error
	Download(in *DonwloadInput) ([]byte, error)
	DoesExist(in *DoesExistInput) (bool, error)
	ReadSize(in *ReadSizeInput) (*types.Size, error)
	ReadOne(in *ReadOneInput) (*types.File, error)
	ReadAll(in *ReadAllInput) ([]*types.File, error)
	Delete(in *DeleteInput) error
}

type UploadInput struct {
	Dir   string
	Name  string
	Bytes []byte
	Size  types.Size
}

type DoesExistInput struct {
	Dir  string
	Name string
}

type DonwloadInput struct {
	Dir  string
	Name string
}

type ReadSizeInput struct {
	Dir  string
	Name string
}

type ReadOneInput struct {
	Dir  string
	Name string
}

type ReadAllInput struct {
	Dir    string
	Prefix string
}

type DeleteInput struct {
	Dir  string
	Name string
}
