package folder

import (
	"fmt"
	"mindia/automation"
	"mindia/policy"
	"mindia/storage"
	"mindia/types"
	"mindia/utils"
	"time"
)

type Automation struct {
	Automation          *automation.Automation
	ApplyToCurrentFiles bool
}

type FolderConfig struct {
	Dir         string           `yaml:"dir"`
	Storage     storage.Storage  `yaml:"storage"`
	Backup      storage.Storage  `yaml:"backup,omitempty"`
	Automations []*Automation    `yaml:"automations"`
	Policies    []*policy.Policy `yaml:"policies"`
}

type Folder struct {
	*FolderConfig `yaml:",inline"`
}

func NewFolder(config *FolderConfig) *Folder {
	f := &Folder{
		FolderConfig: config,
	}
	f.ScheduleBackups()
	f.ApplyAutomationsToCurrentFiles()
	return f
}

func (f *Folder) Upload(name string, bytes []byte) error {
	var err error

	source := automation.Source{
		SourceConfig: &automation.SourceConfig{
			Load: func(Name string) (automation.Body, error) {
				return bytes, nil
			},
		},
	}

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
		_, err = a.Automation.Run(actx, a.Automation.AutomationConfig.Namer, &source, &sinker)
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

	var files []*types.File

	for _, file := range res {
		for _, a := range f.Automations {
			if a.Automation.Namer.IsOf(file.Name) {
				actx := automation.AutomationCtx{
					Name: file.Name,
				}
				outputs, _ := a.Automation.DryRun(actx)
				file.Children = outputs

				for i, child := range file.Children {
					if child == file.Name {
						file.Children = append(file.Children[:i], file.Children[i+1:]...)
						break
					}
				}

				files = append(files, file)
			}
		}
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

func (f *Folder) ScheduleBackups() {
	if f.Backup != nil {
		go func() {
			for {
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
				time.Sleep(60 * time.Second)
			}
		}()
	}
}

func (f *Folder) ApplyAutomationsToCurrentFiles() {
	go func() {
		files, err := f.ReadAll()
		if err != nil {
			return
		}

		for _, file := range files {
			if utils.IsValidUUID(utils.NameWithoutExt(file.Name)) {
				for _, a := range f.Automations {
					if a.ApplyToCurrentFiles {
						source := automation.Source{
							SourceConfig: &automation.SourceConfig{
								Load: func(Name string) (automation.Body, error) {
									return f.Download(Name)
								},
							},
						}

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

						actx := automation.AutomationCtx{
							Name: file.Name,
						}
						_, err = a.Automation.Run(actx, nil, &source, &sinker)
						if err != nil {
							fmt.Printf("Error: %s", err)
							continue
						}

					}
				}
			}
		}
	}()
}
