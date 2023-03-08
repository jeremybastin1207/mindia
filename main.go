package main

import (
	"mindia/apiserver"
	"mindia/backup"
	"mindia/configurer"
	"mindia/folder"
	"mindia/policy"
	"mindia/project"
	"mindia/storage"
	"mindia/synchronizer"
	"mindia/transformer"
	"mindia/utils"
	"os"
	"time"

	"github.com/joho/godotenv"
)

func loadEnv() {
	err := godotenv.Load(".env")
	if err != nil {
		utils.ExitErrorf("Unable to load .env, %v", err)
	}
}

func main() {
	loadEnv()

	filesystemStorage := storage.NewFileSystemStorage(&storage.FilesystemStorageInput{
		MountDir: "./data",
	})
	filesystemBackupStorage := storage.NewFileSystemStorage(&storage.FilesystemStorageInput{
		MountDir: "./data/backup",
	})
	s3Storage := storage.NewS3Storage(&storage.S3StorageInput{
		Client: &storage.S3ClientInput{
			Bucket:          "test-mindia-bucket",
			Region:          "ams3",
			Endpoint:        "https://ams3.digitaloceanspaces.com",
			AccessKeyId:     os.Getenv("ACCESS_KEY_ID"),
			SecretAccessKey: os.Getenv("SECRET_ACCESS_KEY"),
		},
	})
	s3BackupStorage := storage.NewS3Storage(&storage.S3StorageInput{
		Client: &storage.S3ClientInput{
			Bucket:          "test-backup-mindia-bucket",
			Region:          "ams3",
			Endpoint:        "https://ams3.digitaloceanspaces.com",
			AccessKeyId:     os.Getenv("ACCESS_KEY_ID"),
			SecretAccessKey: os.Getenv("SECRET_ACCESS_KEY"),
		},
	})
	filesystemBackup := backup.NewBackup(&backup.BackupInput{
		Target: filesystemBackupStorage,
	})
	s3Backup := backup.NewBackup(&backup.BackupInput{
		Target: s3BackupStorage,
	})
	synchronizer := synchronizer.NewSynchronizer(&synchronizer.SynchronizerInput{
		AutoSync: false,
	})
	apiServer := apiserver.NewApiServer(&apiserver.ApiServerInput{
		Port: 3500,
	})
	resizer := transformer.NewResizer()
	watermarker := transformer.NewWatermarker()

	project := project.NewProject(&project.ProjectInput{
		Name:      "ae",
		ApiServer: apiServer,
	})

	xl_policy := policy.PolicyInput{
		Name:            "xl_thumbnail",
		Width:           280,
		Height:          320,
		TransformerName: resizer.GetName(),
	}
	md_policy := policy.PolicyInput{
		Name:            "md_thumbnail",
		Width:           150,
		Height:          150,
		TransformerName: resizer.GetName(),
	}
	sm_policy := policy.PolicyInput{
		Name:            "sm_thumbnail",
		Width:           80,
		Height:          80,
		TransformerName: resizer.GetName(),
	}

	folder1 := folder.NewFolder(&folder.FolderInput{
		Dir:     "/houses",
		Storage: filesystemStorage,
		Transformers: []transformer.Transformer{
			resizer,
			watermarker,
		},
	})
	folder2 := folder.NewFolder(&folder.FolderInput{
		Dir:     "/houses/garden",
		Storage: filesystemStorage,
		Transformers: []transformer.Transformer{
			resizer,
		},
	})
	folder3 := folder.NewFolder(&folder.FolderInput{
		Dir:     "/users",
		Storage: s3Storage,
		Transformers: []transformer.Transformer{
			resizer,
		},
	})
	folder4 := folder.NewFolder(&folder.FolderInput{
		Dir:     "/users/company",
		Storage: s3Storage,
		Transformers: []transformer.Transformer{
			resizer,
		},
	})

	folder1.WritePolicy(&xl_policy)
	folder2.WritePolicy(&xl_policy)
	folder2.WritePolicy(&md_policy)
	folder3.WritePolicy(&sm_policy)
	folder4.WritePolicy(&md_policy)

	filesystemBackup.AddFolder(folder1)
	filesystemBackup.AddFolder(folder2)
	filesystemBackup.AddFolder(folder3)
	filesystemBackup.AddFolder(folder4)

	s3Backup.AddFolder(folder1)
	s3Backup.AddFolder(folder2)
	s3Backup.AddFolder(folder3)
	s3Backup.AddFolder(folder4)

	synchronizer.AddFolder(folder1)
	synchronizer.AddFolder(folder2)
	synchronizer.AddFolder(folder3)
	synchronizer.AddFolder(folder4)

	project.AddFolder(folder1)
	project.AddFolder(folder2)
	project.AddFolder(folder3)
	project.AddFolder(folder4)

	configurer := configurer.NewConfigurer()
	configurer.PersistConfig(project)

	go func() {
		for {
			synchronizer.Synchronize()
			filesystemBackup.Backup()
			s3Backup.Backup()
			time.Sleep(10 * time.Second)
		}
	}()

	apiServer.Serve()
}
