package synchronizer

import (
	"fmt"
	"mindia/folder"
	"mindia/types"
	"path/filepath"
	"strings"
	"sync"
)

type Synchronizer struct {
	autoSync bool
	folders  folder.FoldersMap
}

type SynchronizerInput struct {
	AutoSync bool
}

func NewSynchronizer(in *SynchronizerInput) *Synchronizer {
	return &Synchronizer{
		autoSync: in.AutoSync,
		folders:  folder.FoldersMap{},
	}
}

func (s *Synchronizer) AddFolder(f *folder.Folder) {
	s.folders[f.Dir] = f
}

func (s *Synchronizer) Synchronize() {
	var wg sync.WaitGroup

	for _, folder := range s.folders {
		files, _ := folder.ReadAll()
		for _, file := range files {
			if folder.IsSourceFile(file) {
				prefix := strings.TrimSuffix(file.Name, filepath.Ext(file.Name)) + "_"
				refs, _ := folder.ReadPrefix(prefix)
				wg.Add(1)
				go s.syncOne(folder, file, refs, &wg)
			}
		}
	}

	wg.Wait()
}

func (s *Synchronizer) syncOne(folder *folder.Folder, source *types.File, refs []*types.File, wg *sync.WaitGroup) {
	defer wg.Done()
	needSync := false

	policies, err := folder.ReadPolicies()
	if err != nil {
		fmt.Printf("Error: %s", err)
		return
	}

	for _, ref := range refs {
		obj, err := folder.ReadSize(ref.Dir, ref.Name)
		if err != nil {
			fmt.Printf("Error: %s", err)
			return
		}

		for _, policy := range policies {
			if policy.IsOf(ref.Name) {
				if obj.Width != policy.Width && obj.Height != policy.Height {
					needSync = true
				}
				continue
			}
		}
	}

	if !needSync {
		for _, policy := range policies {
			found := false
			for _, ref := range refs {
				if policy.IsOf(ref.Name) {
					found = true
					break
				}
			}
			if !found {
				needSync = true
			}
		}
	}

	if needSync {
		bytes, _ := folder.Download(source.Name)
		folder.Upload(source.Name, bytes)
	}
}
