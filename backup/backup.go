package backup

import (
	"fmt"
	"mindia/folder"
	"mindia/storage"
	"sync"
)

type Backup struct {
	folders folder.FoldersMap
	target  storage.Storage
}

type BackupInput struct {
	Target storage.Storage
}

func NewBackup(in *BackupInput) *Backup {
	return &Backup{
		folders: folder.FoldersMap{},
		target:  in.Target,
	}
}

func (b *Backup) AddFolder(f *folder.Folder) {
	b.folders[f.Dir] = f
}

func (b *Backup) Backup() {
	wg := new(sync.WaitGroup)
	wg.Add(1)
	go func() {
		for _, f := range b.folders {
			files, err := f.ReadAll()
			if err != nil {
				fmt.Println("Error: unable to perform backup", err)
				continue
			}
			for _, file := range files {
				if f.IsSourceFile(file) {
					bytes, err := f.Download(file.Name)
					if err != nil {
						continue
					}
					doesExist, _ := b.target.DoesExist(&storage.DoesExistInput{Dir: f.Dir, Name: file.Name})
					if !doesExist {
						b.target.Upload(&storage.UploadInput{
							Dir:   f.Dir,
							Name:  file.Name,
							Bytes: bytes,
						})
					}
				}
			}
		}
		wg.Done()
	}()
	wg.Wait()
}
