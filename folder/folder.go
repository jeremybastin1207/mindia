package folder

import (
	"fmt"
	"mindia/automation"
	"mindia/storage"
	"mindia/types"
	"mindia/utils"
	"path/filepath"
)

type FolderConfig struct {
	Dir         string                   `yaml:"dir"`
	Storage     storage.Storage          `yaml:"storage"`
	Automations []*automation.Automation `yaml:"automations"`
}

type Folder struct {
	*FolderConfig `yaml:",inline"`
}

func NewFolder(config *FolderConfig) *Folder {
	return &Folder{
		FolderConfig: config,
	}
}

func (f *Folder) IsSourceFile(file *types.File) bool {
	return utils.IsValidUUID(utils.NameWithoutExt(file.Name))
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

	for _, a := range f.Automations {
		ctx, b, err := a.Run(name, bytes)
		if err != nil {
			fmt.Printf("Error: %s", err)
			continue
		}

		name := ctx.Value(automation.NamerCtxKey{}).(string)
		size := ctx.Value(automation.ResizerCtxKey{}).(types.Size)

		fmt.Println(name)
		fmt.Println(size)

		f.Storage.Upload(&storage.UploadInput{
			Dir:   f.Dir,
			Name:  name,
			Bytes: b,
			Size: types.Size{
				Width:  size.Width,
				Height: size.Height,
			},
		})
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
