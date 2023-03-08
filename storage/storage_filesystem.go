package storage

import (
	"errors"
	"image"
	_ "image/gif"
	_ "image/jpeg"
	_ "image/png"
	"mindia/types"
	"mindia/utils"
	"os"
	"path/filepath"
	"strings"
)

type FilesystemStorage struct {
	MountDir string
}

type FilesystemStorageInput struct {
	MountDir string
}

func NewFileSystemStorage(p *FilesystemStorageInput) *FilesystemStorage {
	s := &FilesystemStorage{
		MountDir: p.MountDir,
	}
	s.createMountPathNotExists("")
	return s
}

func (s *FilesystemStorage) createMountPathNotExists(dir string) {
	err := os.MkdirAll(utils.JoinPath(s.MountDir, dir), 0777)
	if err != nil {
		utils.ExitErrorf("Unable to create dir, %v", err)
	}
}

func (s *FilesystemStorage) Upload(in *UploadInput) error {
	s.createMountPathNotExists(in.Dir)
	return os.WriteFile(utils.JoinPath(s.MountDir, in.Dir, in.Name), in.Bytes, 0777)
}

func (s *FilesystemStorage) Download(in *DonwloadInput) ([]byte, error) {
	s.createMountPathNotExists(in.Dir)
	bytes, err := os.ReadFile(utils.JoinPath(s.MountDir, in.Dir, in.Name))
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return nil, nil
		}
		return nil, err
	}
	return bytes, nil
}

func (s *FilesystemStorage) DoesExist(in *DoesExistInput) (bool, error) {
	file, err := os.Open(utils.JoinPath(s.MountDir, in.Dir, in.Name))
	if err != nil {
		return false, err
	}
	return file != nil, nil
}

func (s *FilesystemStorage) ReadSize(in *ReadSizeInput) (*types.Size, error) {
	file, err := os.Open(utils.JoinPath(s.MountDir, in.Dir, in.Name))
	if err != nil {
		return nil, err
	}
	defer file.Close()

	img, _, err := image.DecodeConfig(file)
	if err != nil {
		return nil, err
	}
	return &types.Size{
		Width:  int32(img.Width),
		Height: int32(img.Height),
	}, nil
}

func (s *FilesystemStorage) ReadOne(in *ReadOneInput) (*types.File, error) {
	s.createMountPathNotExists(in.Dir)
	_, err := os.ReadFile(utils.JoinPath(s.MountDir, in.Dir, in.Name))
	if err != nil {
		return nil, err
	}
	return &types.File{
		Dir:  in.Dir,
		Name: in.Name,
	}, nil
}

func (s *FilesystemStorage) ReadAll(in *ReadAllInput) ([]*types.File, error) {
	s.createMountPathNotExists(in.Dir)
	var files, err = os.ReadDir(utils.JoinPath(s.MountDir, in.Dir))
	if err != nil {
		return nil, err
	}
	var files2 []*types.File
	for _, f := range files {
		if in.Prefix != "" {
			if !strings.HasPrefix(f.Name(), in.Prefix) {
				continue
			}
		}
		files2 = append(files2, &types.File{
			Dir:  in.Dir,
			Name: f.Name(),
		})
	}
	return files2, nil
}

func (s *FilesystemStorage) Delete(in *DeleteInput) error {
	s.createMountPathNotExists(in.Dir)
	return os.Remove(filepath.Join(s.MountDir, in.Dir, in.Name))
}

type FilesystemStorageConfig struct {
	MountDir string
}

func (s *FilesystemStorage) Config() FilesystemStorageConfig {
	return FilesystemStorageConfig{
		MountDir: s.MountDir,
	}
}
