package storage

import (
	"bytes"
	"io"
	"mindia/utils"

	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/credentials"
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/s3"
)

type S3Object struct {
	Key      string
	Metadata map[string]*string
}

type S3ClientInput struct {
	Bucket          string
	AccessKeyId     string
	SecretAccessKey string
	Endpoint        string
	Region          string
}

type S3Client struct {
	Bucket string
	s3     *s3.S3
}

func NewS3Client(in *S3ClientInput) *S3Client {
	s3 := S3Client{
		Bucket: in.Bucket,
	}
	s3.createSession(in)
	return &s3
}

func (s *S3Client) createSession(in *S3ClientInput) {
	s3Config := &aws.Config{
		Credentials: credentials.NewStaticCredentials(in.AccessKeyId, in.SecretAccessKey, ""),
		Endpoint:    aws.String(in.Endpoint),
		Region:      aws.String(in.Region),
	}
	newSession, err := session.NewSession(s3Config)
	if err != nil {
		utils.ExitErrorf("Unable create a new session, %v", err)
	}
	s.s3 = s3.New(newSession)
}

func (s *S3Client) CreateBucket(name string) error {
	params := &s3.CreateBucketInput{
		Bucket: aws.String(name),
	}
	_, err := s.s3.CreateBucket(params)
	return err
}

func (s *S3Client) DeleteBucket(name string) error {
	params := &s3.DeleteBucketInput{
		Bucket: aws.String(name),
	}
	_, err := s.s3.DeleteBucket(params)
	return err
}

func (s *S3Client) ListBucketNames() ([]string, error) {
	result, err := s.s3.ListBuckets(nil)
	if err != nil {
		return nil, err
	}
	var names []string
	for _, b := range result.Buckets {
		names = append(names, *b.Name)
	}
	return names, nil
}

type ListObjectsParams struct {
	Bucket string
	Prefix string
}

func (s *S3Client) ListObjects(p *ListObjectsParams) ([]S3Object, error) {
	intput := &s3.ListObjectsV2Input{
		Bucket: aws.String(p.Bucket),
		Prefix: aws.String(p.Prefix),
	}
	output, err := s.s3.ListObjectsV2(intput)
	if err != nil {
		return nil, err
	}
	var objs []S3Object
	for _, obj := range output.Contents {
		objs = append(objs, S3Object{
			Key:      *obj.Key,
			Metadata: nil,
		})
	}
	return objs, nil
}

type GetObjectParams struct {
	Bucket string
	Key    string
}

func (s *S3Client) DownloadObject(p *GetObjectParams) ([]byte, error) {
	input := &s3.GetObjectInput{
		Bucket: aws.String(p.Bucket),
		Key:    aws.String(p.Key),
	}
	output, err := s.s3.GetObject(input)
	if err != nil {
		return nil, err
	}
	defer output.Body.Close()
	return io.ReadAll(output.Body)
}

func (s *S3Client) GetObject(p *GetObjectParams) (*S3Object, error) {
	input := &s3.GetObjectInput{
		Bucket: aws.String(p.Bucket),
		Key:    aws.String(p.Key),
	}
	output, err := s.s3.GetObject(input)
	if err != nil {
		return nil, err
	}

	return &S3Object{
		Key:      p.Key,
		Metadata: output.Metadata,
	}, nil
}

type PutObjectParams struct {
	Bucket   string
	Key      string
	Body     []byte
	Metadata map[string]*string
}

func (s *S3Client) PutObject(p *PutObjectParams) error {
	input := &s3.PutObjectInput{
		Bucket:   aws.String(p.Bucket),
		Key:      aws.String(p.Key),
		ACL:      aws.String("public-read"),
		Body:     bytes.NewReader(p.Body),
		Metadata: p.Metadata,
	}
	_, err := s.s3.PutObject(input)
	return err
}

type DeleteObjectParams struct {
	Bucket string
	Key    string
}

func (s *S3Client) DeleteObject(p *DeleteObjectParams) error {
	input := &s3.DeleteObjectInput{
		Bucket: aws.String(p.Bucket),
		Key:    aws.String(p.Key),
	}
	_, err := s.s3.DeleteObject(input)
	return err
}
