package folder

import (
	"context"
	"mindia/policy"
	"mindia/storage"
	"mindia/transformer"
	"mindia/types"
	"mindia/utils"
	"path/filepath"
)

type Folder struct {
	Dir          string
	Storage      storage.Storage
	Policies     policy.PoliciesMap
	Transformers transformer.TransformersMap
}

type FolderInput struct {
	Dir          string
	Storage      storage.Storage
	Policies     policy.PoliciesMap
	Transformers []transformer.Transformer
}

func NewFolder(in *FolderInput) *Folder {
	if in.Policies == nil {
		in.Policies = policy.PoliciesMap{}
	}
	transformers := transformer.TransformersMap{}
	if in.Transformers != nil {
		for _, t := range in.Transformers {
			transformers[t.GetName()] = t
		}
	}
	f := &Folder{
		Dir:          in.Dir,
		Storage:      in.Storage,
		Policies:     in.Policies,
		Transformers: transformers,
	}
	return f
}

func (f *Folder) IsSourceFile(file *types.File) bool {
	return utils.IsValidUUID(utils.NameWithoutExt(file.Name))
}

func (f *Folder) WritePolicy(in *policy.PolicyInput) error {
	p := policy.NewPolicy(in)
	f.Policies[p.Name] = &p
	return nil
}

func (f *Folder) ReadPolicy(name string) (*policy.Policy, error) {
	return f.Policies[name], nil
}

func (f *Folder) ReadPolicies() (policy.PoliciesMap, error) {
	return f.Policies, nil
}

func (f *Folder) Upload(name string, bytes []byte) (*types.File, error) {
	if !utils.IsValidUUID(utils.NameWithoutExt(name)) {
		name = types.GenerateName(name)
	}
	err := f.Storage.Upload(&storage.UploadInput{
		Dir:   f.Dir,
		Name:  name,
		Bytes: bytes,
	})
	if err != nil {
		return nil, err
	}

	for _, p := range f.Policies {
		transformer := f.Transformers[p.TransformerName]

		ctx := context.Background()
		ctx = context.WithValue(ctx, "size", types.Size{
			Width:  p.Width,
			Height: p.Height,
		})
		b, _ := transformer.Transform(ctx, bytes)

		f.Storage.Upload(&storage.UploadInput{
			Dir:   f.Dir,
			Name:  p.GetName(name),
			Bytes: b,
			Size: types.Size{
				Width:  p.Width,
				Height: p.Height,
			},
		})
	}
	if err != nil {
		return nil, err
	}

	return f.Storage.ReadOne(&storage.ReadOneInput{
		Dir:  f.Dir,
		Name: name,
	})
}

func (f *Folder) ReadSize(dir, name string) (*types.Size, error) {
	return f.Storage.ReadSize(&storage.ReadSizeInput{
		Dir:  dir,
		Name: name,
	})
}

func (f *Folder) ReadOne(dir string) (*types.File, error) {
	return f.Storage.ReadOne(&storage.ReadOneInput{
		Dir:  f.Dir,
		Name: types.GetName(dir),
	})
}

func (f *Folder) ReadAll() ([]*types.File, error) {
	res, err := f.Storage.ReadAll(&storage.ReadAllInput{Dir: f.Dir})

	files := []*types.File{}
	for _, f := range res {
		if filepath.Ext(f.Name) != ".jpg" && filepath.Ext(f.Name) != ".jpeg" {
			continue
		}
		files = append(files, f)
	}
	return files, err
}

func (f *Folder) ReadPrefix(prefix string) ([]*types.File, error) {
	return f.Storage.ReadAll(&storage.ReadAllInput{
		Dir:    f.Dir,
		Prefix: prefix,
	})
}

func (f *Folder) Download(name string) ([]byte, error) {
	return f.Storage.Download(&storage.DonwloadInput{
		Dir:  f.Dir,
		Name: name,
	})
}

func (f *Folder) DeleteOne(name string) error {
	baseName := utils.NameWithoutExt(name)

	files, err := f.ReadPrefix(baseName)
	if err != nil {
		return err
	}

	for _, file := range files {
		f.Storage.Delete(&storage.DeleteInput{
			Dir:  f.Dir,
			Name: file.Name,
		})
	}
	return nil
}
