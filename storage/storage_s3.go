package storage

import (
	"mindia/types"
	"mindia/utils"
	"strconv"
	"strings"

	"github.com/aws/aws-sdk-go/aws/awserr"
	"github.com/aws/aws-sdk-go/service/s3"
)

type S3StorageConfig struct {
	*S3ClientConfig `yaml:",inline"`
}

type S3Storage struct {
	*S3StorageConfig `yaml:",inline"`
	s3               *S3Client
}

func NewS3Storage(config *S3StorageConfig) *S3Storage {
	return &S3Storage{
		s3: NewS3Client(config.S3ClientConfig),
	}
}

func metadataToInt32(obj *S3Object, key string) int32 {
	val := obj.Metadata[key]
	if val == nil {
		return 0
	}
	parsedVal, _ := strconv.ParseInt(*val, 10, 32)
	return int32(parsedVal)
}

func (s *S3Storage) Upload(in *UploadInput) error {
	width := strconv.Itoa(int(in.Size.Width))
	height := strconv.Itoa(int(in.Size.Height))

	return s.s3.PutObject(&PutObjectParams{
		Bucket: s.s3.Bucket,
		Key:    utils.JoinPath(in.Dir, in.Name),
		Body:   in.Bytes,
		Metadata: map[string]*string{
			"width":  &width,
			"height": &height,
		},
	})
}

func (s *S3Storage) Download(in *DonwloadInput) ([]byte, error) {
	bytes, err := s.s3.DownloadObject(&GetObjectParams{
		Bucket: s.s3.Bucket,
		Key:    utils.JoinPath(in.Dir, in.Name),
	})
	if aerr, ok := err.(awserr.Error); ok {
		if aerr.Code() == s3.ErrCodeNoSuchKey {
			return nil, nil
		}
		return nil, err
	}
	return bytes, err
}

func (s *S3Storage) ReadSize(in *ReadSizeInput) (*types.Size, error) {
	obj, err := s.s3.GetObject(&GetObjectParams{
		Bucket: s.s3.Bucket,
		Key:    strings.TrimPrefix(utils.JoinPath(in.Dir, in.Name), "/"),
	})
	if err != nil {
		return nil, err
	}

	return &types.Size{
		Width:  metadataToInt32(obj, "Width"),
		Height: metadataToInt32(obj, "Height"),
	}, nil
}

func (s *S3Storage) DoesExist(in *DoesExistInput) (bool, error) {
	_, err := s.ReadOne(&ReadOneInput{
		Dir:  in.Dir,
		Name: in.Name,
	})
	if aerr, ok := err.(awserr.Error); ok {
		if aerr.Code() == s3.ErrCodeNoSuchKey {
			return false, nil
		}
		return false, err
	}
	return true, nil
}

func (s *S3Storage) ReadOne(in *ReadOneInput) (*types.File, error) {
	_, err := s.s3.GetObject(&GetObjectParams{
		Bucket: s.s3.Bucket,
		Key:    utils.JoinPath(in.Dir, in.Name),
	})
	if err != nil {
		return nil, err
	}
	return &types.File{
		Dir:  in.Dir,
		Name: in.Name,
	}, nil
}

func (s *S3Storage) ReadAll(in *ReadAllInput) ([]*types.File, error) {
	p := &ListObjectsParams{Bucket: s.s3.Bucket}
	if in.Prefix != "" {
		p.Prefix = strings.TrimPrefix(utils.JoinPath(in.Dir, in.Prefix), "/")
	}
	res, err := s.s3.ListObjects(p)
	if err != nil {
		return nil, err
	}
	var files []*types.File
	for _, obj := range res {
		if strings.Contains(strings.TrimPrefix(strings.TrimPrefix(obj.Key, strings.TrimPrefix(in.Dir, "/")), "/"), "/") {
			continue
		}
		files = append(files, &types.File{
			Dir:  in.Dir,
			Name: types.GetName(obj.Key),
		})
	}
	return files, nil
}

func (s *S3Storage) Delete(in *DeleteInput) error {
	return s.s3.DeleteObject(&DeleteObjectParams{
		Bucket: s.s3.Bucket,
		Key:    utils.JoinPath(in.Dir, in.Name),
	})
}
