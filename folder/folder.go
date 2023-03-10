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
	Backup      storage.Storage          `yaml:"backup,omitempty"`
	Automations []*automation.Automation `yaml:"automations"`
}

type Folder struct {
	*FolderConfig `yaml:",inline"`
}

func NewFolder(config *FolderConfig) *Folder {
	f := &Folder{
		FolderConfig: config,
	}
	/*
		if config.Backup != nil {
			go func() {
				for {
					f.backup()
					f.sync()
					time.Sleep(10 * time.Second)
				}
			}()
		} */

	return f
}

func (f *Folder) Upload(name string, bytes []byte) error {
	var err error

	sinker := automation.Sinker{
		SinkerConfig: &automation.SinkerConfig{
			Sink: func(actx automation.AutomationCtx) {
				f.Storage.Upload(&storage.UploadInput{
					Dir:   f.Dir,
					Name:  actx.Name,
					Bytes: actx.Body,
				})
			},
		},
	}

	for _, a := range f.Automations {
		actx := automation.AutomationCtx{
			Name: name,
			Body: bytes,
		}
		err = a.Run(actx, sinker)
		if err != nil {
			fmt.Printf("Error: %s", err)
			continue
		}
	}

	return nil
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

/* func (f *Folder) backup() {
	files, _ := f.ReadAll()
	for _, file := range files {
		if types.IsSourceFile(file) {
			bytes, err := f.Download(file.Name)
			if err != nil {
				return
			}
			doesExist, _ := f.Backup.DoesExist(&storage.DoesExistInput{
				Dir:  f.Dir,
				Name: file.Name,
			})
			if !doesExist {
				f.Backup.Upload(&storage.UploadInput{
					Dir:   f.Dir,
					Name:  file.Name,
					Bytes: bytes,
				})
			}
		}
	}
} */

func (f *Folder) sync() {
	/* 	for _, a := range f.Automations {
		if a.Triggers.SyncCron != "" {
			files, _ := f.ReadAll()
			for _, file := range files {
				if !a.Namer.Match(file.Name) {
					continue
				}
				bytes, _ := f.Download(file.Name)
				isSync, err := a.IsSync(file, bytes)
				if err != nil {
					continue
				}
				if !isSync {
					fmt.Println("sync")
					files2, _ := f.ReadPrefix(a.Namer.Prefix(file.Name))
					for _, file2 := range files2 {
						if types.IsSourceFile(file2) {
							bytes, _ := f.Download(file2.Name)
							f.Upload(file2.Name, bytes)
							break
						}
					}
				}
			}
		}
	} */
}
