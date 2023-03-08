package storage

import "mindia/types"

type DropboxStorage struct {
}

func (s *DropboxStorage) Upload(in *UploadInput) error {
	return nil
}

func (s *DropboxStorage) Download(in *DonwloadInput) ([]byte, error) {
	return nil, nil
}

func (s *DropboxStorage) DoesExist(in *DoesExistInput) (bool, error) {
	return false, nil
}

func (s *DropboxStorage) ReadSize(in *ReadSizeInput) (*types.Size, error) {
	return nil, nil
}

func (s *DropboxStorage) ReadOne(in *ReadOneInput) (*types.File, error) {
	return nil, nil
}

func (s *DropboxStorage) ReadAll(in *ReadAllInput) ([]*types.File, error) {
	return nil, nil
}

func (s *DropboxStorage) Delete(in *DeleteInput) error {
	return nil
}
